use crate::models::*;
use crate::queries;
use crate::rendering::RenderedBalance;
use crate::schema::*;
use crate::web::pagination::*;
use crate::web::user::UserId;
use crate::web::util::{log_error_and_500, InfoResponse, QueryRequest, QueryResponse};
use crate::web::DbConn;

use diesel::prelude::*;
use log::warn;
use rocket::http::Status;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

use std::collections::HashMap;

#[post("/balances/query", data = "<request>")]
pub async fn query(
    uid: UserId,
    connection: DbConn,
    request: Json<QueryRequest>,
) -> Result<Json<QueryResponse<RenderedBalance>>, Status> {
    connection
        .run(move |c| {
            let request = request.0;

            if request.rows_per_page > 100 {
                warn!(
                    "Request with too many rows per page ({} > 100)",
                    request.rows_per_page
                );
                return Err(Status::BadRequest);
            }

            if request.rows_per_page <= 0 || request.page < 0 {
                warn!("Request with invalid paging settings");
                return Err(Status::BadRequest);
            }

            let mut query = balances::table
                .left_join(
                    account_synchronizations::table.on((account_synchronizations::account1
                        .eq(balances::account_id)
                        .and(account_synchronizations::user1.ne(*uid)))
                    .or(account_synchronizations::account2
                        .eq(balances::account_id)
                        .and(account_synchronizations::user2.ne(*uid)))),
                )
                .left_join(accounts::table.on(accounts::id.eq(balances::account_id)))
                .filter(
                    accounts::user_id
                        .eq(*uid)
                        .or(account_synchronizations::user1.eq(*uid))
                        .or(account_synchronizations::user2.eq(*uid)),
                )
                .distinct()
                .select((
                    balances::all_columns,
                    account_synchronizations::all_columns.nullable(),
                ))
                .into_boxed();

            if let Some(from) = request.from {
                query = query.filter(balances::date.ge(from));
            }
            if let Some(to) = request.to {
                query = query.filter(balances::date.le(to));
            }

            for sb in request.sort_by.iter() {
                let asc = sb.direction.to_lowercase() == "ascending";

                match sb.column.as_ref() {
                    "amount" => {
                        if asc {
                            query = query.then_order_by(balances::amount.asc());
                        } else {
                            query = query.then_order_by(balances::amount.desc());
                        }
                    }
                    "id" => {
                        if asc {
                            query = query.then_order_by(balances::id.asc());
                        } else {
                            query = query.then_order_by(balances::id.desc());
                        }
                    }
                    "comment" => {
                        if asc {
                            query = query.then_order_by(balances::comment.asc());
                        } else {
                            query = query.then_order_by(balances::comment.desc());
                        }
                    }
                    "date" => {
                        if asc {
                            query = query.then_order_by(balances::date.asc());
                        } else {
                            query = query.then_order_by(balances::date.desc());
                        }
                    }
                    _ => {
                        return Err(Status::BadRequest);
                    }
                }
            }

            query = query.then_order_by(balances::id.asc());

            for (column, values) in request.filter_by.iter() {
                // TODO: filtering by account?

                match column.as_ref() {
                    "accountId" | "account_id" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<i32>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(
                            balances::account_id.eq_any(values_parsed.clone()).or(
                                account_synchronizations::account1
                                    .eq_any(values_parsed.clone())
                                    .or(account_synchronizations::account2.eq_any(values_parsed)),
                            ),
                        );
                    }
                    "id" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<i32>>>()
                            .ok_or(Status::BadRequest)?;

                        query = query.filter(balances::id.eq_any(values_parsed));
                    }
                    "amount" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<i64>>>()
                            .ok_or(Status::BadRequest)?;

                        query = query.filter(balances::amount.eq_any(values_parsed));
                    }
                    "comment" => {
                        query = query.filter(balances::comment.eq_any(values));
                    }
                    _ => {
                        return Err(Status::BadRequest);
                    }
                }
            }

            if let Some(s) = request.needle {
                // TODO: more fields to search in (amount would be nice ...)

                let s = format!("%{}%", s.trim());
                if !s.is_empty() {
                    query = query.filter(balances::comment.ilike(s.clone()));
                }
            }

            let (data, row_count) = query
                .paginate(request.page)
                .per_page(request.rows_per_page)
                .load_and_count::<(Balance, Option<AccountSynchronization>)>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let rbals = data
                .into_iter()
                .map(|(b, acs)| RenderedBalance::render(b, acs))
                .collect::<Vec<_>>();

            Ok(Json(QueryResponse {
                records: rbals,
                filtered_record_count: row_count,
                total_record_count: if request.filter_by.is_empty()
                    && request.from.is_none()
                    && request.to.is_none()
                {
                    Some(row_count)
                } else {
                    None
                },
            }))
        })
        .await
}

#[get("/balances/info")]
pub async fn info(uid: UserId, connection: DbConn) -> Result<Json<InfoResponse>, Status> {
    connection
        .run(move |c| {
            let total_record_count = balances::table
                .select(diesel::dsl::count(balances::id))
                .filter(
                    balances::id.eq_any(
                        balances::table
                            .select(balances::id)
                            .distinct_on(balances::id)
                            .left_join(
                                account_synchronizations::table.on(balances::account_id
                                    .eq(account_synchronizations::account1)
                                    .or(
                                        balances::account_id.eq(account_synchronizations::account2)
                                    )),
                            )
                            .inner_join(accounts::table.on(accounts::id.eq(balances::account_id)))
                            .filter(
                                accounts::user_id
                                    .eq(*uid)
                                    .or(account_synchronizations::user1.eq(*uid))
                                    .or(account_synchronizations::user2.eq(*uid)),
                            ),
                    ),
                )
                .get_result::<i64>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            Ok(Json(InfoResponse {
                total_record_count,
                filter_hints: HashMap::<String, Vec<String>>::new(),
            }))
        })
        .await
}

#[get("/balances?<offset>&<count>")]
pub async fn list(
    uid: UserId,
    connection: DbConn,
    offset: Option<i64>,
    count: Option<i64>,
) -> Result<Json<Vec<RenderedBalance>>, Status> {
    connection
        .run(move |c| {
            let bals = queries::relevant_balances(c, *uid, offset, count)
                .map_err(|e| log_error_and_500(Box::new(e)))?
                .into_iter()
                .map(|(balance, syncing)| RenderedBalance::render(balance, syncing))
                .collect();

            Ok(Json(bals))
        })
        .await
}

#[get("/balances/<id>")]
pub async fn get(
    uid: UserId,
    connection: DbConn,
    id: i32,
) -> Result<Json<RenderedBalance>, Status> {
    connection
        .run(move |c| {
            let (balance, syncing) = queries::relevant_balance_by_id(c, *uid, id)
                .map_err(|e| log_error_and_500(Box::new(e)))?
                .ok_or(Status::NotFound)?;

            Ok(Json(RenderedBalance::render(balance, syncing)))
        })
        .await
}

// #[delete("/balances/<id>")]
// pub fn delete(uid: UserId, connection: DbConn, id: i32) -> Result<(), Status> {
//     todo!() // TODO
// }

// #[put("/balances/<id>", data = "<balances>")]
// pub fn update(
//     uid: UserId,
//     connection: DbConn,
//     id: i32,
//     account: Json<RenderedBalance>,
// ) -> Result<(), Status> {
//     todo!() // TODO
// }

// #[post("/balances", data = "<balances>")]
// pub fn create(
//     uid: UserId,
//     connection: DbConn,
//     account: Json<NewBalance>,
// ) -> Result<Json<RenderedBalance>, Status> {
//     todo!() // TODO
// }
