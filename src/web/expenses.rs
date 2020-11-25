use crate::models::*;
use crate::queries;
use crate::rendering::RenderedExpense;
use crate::schema::*;
use crate::web::pagination::*;
use crate::web::user::UserId;
use crate::web::util::{log_error_and_500, InfoResponse, QueryRequest, QueryResponse};
use crate::web::DbConn;

use chrono::{Duration, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use log::warn;
use rocket::http::Status;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

use std::collections::HashMap;

#[post("/expenses/query", data = "<request>")]
pub async fn query(
    uid: UserId,
    connection: DbConn,
    request: Json<QueryRequest>,
) -> Result<Json<QueryResponse<RenderedExpense>>, Status> {
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

            let mut query = expenses::table
                .select(expenses::all_columns)
                .distinct()
                .left_join(
                    expense_transactions::table
                        .on(expense_transactions::expense_id.eq(expenses::id)),
                )
                .left_join(
                    account_synchronizations::table.on(expense_transactions::account_id
                        .eq(account_synchronizations::account1)
                        .or(
                            expense_transactions::account_id.eq(account_synchronizations::account2)
                        )),
                )
                .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
                .filter(
                    accounts::user_id
                        .eq(*uid)
                        .or(account_synchronizations::user1.eq(*uid))
                        .or(account_synchronizations::user2.eq(*uid)),
                )
                .left_join(
                    expense_categories::table.on(expense_categories::expense_id.eq(expenses::id)),
                )
                .left_join(
                    category_replacements::table.on(category_replacements::replacement
                        .eq(expense_categories::category_id)
                        .and(category_replacements::user_id.eq(*uid))),
                )
                .filter(expenses::is_deleted.eq(false))
                .into_boxed();

            if let Some(from) = request.from {
                query = query.filter(expenses::booking_end.ge(from));
            }
            if let Some(to) = request.to {
                query = query.filter(expenses::booking_start.le(to));
            }

            query = query
                .then_order_by(expenses::is_template.asc())
                .then_order_by(expenses::is_unchecked.desc());

            for sb in request.sort_by.iter() {
                let asc = sb.direction.to_lowercase() == "ascending";
                // TODO: sorting by amount would be nice

                match sb.column.as_ref() {
                    "info.title" => {
                        if asc {
                            query = query.then_order_by(expenses::title.asc());
                        } else {
                            query = query.then_order_by(expenses::title.desc());
                        }
                    }
                    "info.id" => {
                        if asc {
                            query = query.then_order_by(expenses::id.asc());
                        } else {
                            query = query.then_order_by(expenses::id.desc());
                        }
                    }
                    "info.store" => {
                        if asc {
                            query = query.then_order_by(expenses::store.asc());
                        } else {
                            query = query.then_order_by(expenses::store.desc());
                        }
                    }
                    "info.description" => {
                        if asc {
                            query = query.then_order_by(expenses::description.asc());
                        } else {
                            query = query.then_order_by(expenses::description.desc());
                        }
                    }
                    "info.comments" => {
                        if asc {
                            query = query.then_order_by(expenses::comments.asc());
                        } else {
                            query = query.then_order_by(expenses::comments.desc());
                        }
                    }
                    "info.bookingStart" | "info.booking_start" => {
                        if asc {
                            query = query.then_order_by(expenses::booking_start.asc());
                        } else {
                            query = query.then_order_by(expenses::booking_start.desc());
                        }
                    }
                    "info.bookingEnd" | "info.booking_end" => {
                        if asc {
                            query = query.then_order_by(expenses::booking_end.asc());
                        } else {
                            query = query.then_order_by(expenses::booking_end.desc());
                        }
                    }
                    "info.isTemplate" | "info.is_template" => {
                        if asc {
                            query = query.then_order_by(expenses::is_template.asc());
                        } else {
                            query = query.then_order_by(expenses::is_template.desc());
                        }
                    }
                    "info.isPreliminary" | "info.is_preliminary" => {
                        if asc {
                            query = query.then_order_by(expenses::is_preliminary.asc());
                        } else {
                            query = query.then_order_by(expenses::is_preliminary.desc());
                        }
                    }
                    "info.isTaxRelevant" | "info.is_tax_relevant" => {
                        if asc {
                            query = query.then_order_by(expenses::is_tax_relevant.asc());
                        } else {
                            query = query.then_order_by(expenses::is_tax_relevant.desc());
                        }
                    }
                    "info.isUnchecked" | "info.is_unchecked" => {
                        if asc {
                            query = query.then_order_by(expenses::is_unchecked.asc());
                        } else {
                            query = query.then_order_by(expenses::is_unchecked.desc());
                        }
                    }
                    _ => {
                        return Err(Status::BadRequest);
                    }
                }
            }

            query = query.then_order_by(expenses::id.asc());

            for (column, values) in request.filter_by.iter() {
                match column.as_ref() {
                    "transactions" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<i32>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(
                            expense_transactions::account_id
                                .eq_any(values_parsed.clone())
                                .or(account_synchronizations::account1
                                    .eq_any(values_parsed.clone())
                                    .or(account_synchronizations::account2.eq_any(values_parsed))),
                        );
                    }
                    "categories" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<i32>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(
                            expense_categories::category_id
                                .eq_any(values_parsed.clone())
                                .or(category_replacements::original.eq_any(values_parsed.clone())),
                        );
                    }
                    "info.title" => {
                        query = query.filter(expenses::title.eq_any(values));
                    }
                    "info.id" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<i32>>>()
                            .ok_or(Status::BadRequest)?;

                        query = query.filter(expenses::id.eq_any(values_parsed));
                    }
                    "info.store" => {
                        query = query.filter(expenses::store.eq_any(values));
                    }
                    "info.description" => {
                        query = query.filter(expenses::description.eq_any(values));
                    }
                    "info.comments" => {
                        query = query.filter(expenses::comments.eq_any(values));
                    }
                    "info.isTemplate" | "info.is_template" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<bool>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(expenses::is_template.eq_any(values_parsed));
                    }
                    "info.isPreliminary" | "info.is_preliminary" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<bool>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(expenses::is_preliminary.eq_any(values_parsed));
                    }
                    "info.isTaxRelevant" | "info.is_tax_relevant" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<bool>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(expenses::is_tax_relevant.eq_any(values_parsed));
                    }
                    "info.isUnchecked" | "info.is_unchecked" => {
                        let values_parsed = values
                            .into_iter()
                            .map(|s| s.parse().ok())
                            .collect::<Option<Vec<bool>>>()
                            .ok_or(Status::BadRequest)?;
                        query = query.filter(expenses::is_unchecked.eq_any(values_parsed));
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
                    query = query.filter(
                        expenses::store
                            .ilike(s.clone())
                            .or(expenses::description.ilike(s.clone()))
                            .or(expenses::title.ilike(s.clone()))
                            .or(expenses::comments.ilike(s)),
                    );
                }
            }

            let (expenses, row_count) = query
                .paginate(request.page)
                .per_page(request.rows_per_page)
                .load_and_count::<Expense>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let exp_ids = expenses.iter().map(|e| e.id).collect::<Vec<_>>();

            let transactions = expense_transactions::table
                .inner_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
                .left_join(
                    account_synchronizations::table.on((account_synchronizations::account1
                        .eq(expense_transactions::account_id)
                        .and(account_synchronizations::user1.ne(*uid)))
                    .or(account_synchronizations::account2
                        .eq(expense_transactions::account_id)
                        .and(account_synchronizations::user2.ne(*uid)))),
                )
                .filter(
                    expense_transactions::expense_id
                        .eq_any(exp_ids.iter().cloned().collect::<Vec<_>>()),
                )
                .load::<(ExpenseTransaction, Account, Option<AccountSynchronization>)>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let categories = expense_categories::table
                .left_join(
                    category_replacements::table.on(category_replacements::original
                        .eq(expense_categories::category_id)
                        .and(category_replacements::user_id.eq(*uid))),
                )
                .filter(
                    expense_categories::expense_id
                        .eq_any(exp_ids.iter().cloned().collect::<Vec<_>>()),
                )
                .load::<(ExpenseCategory, Option<CategoryReplacement>)>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let receipts = expense_receipts::table
                .filter(
                    expense_receipts::expense_id
                        .eq_any(exp_ids.iter().cloned().collect::<Vec<_>>()),
                )
                .load::<ExpenseReceipt>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let events = expense_events::table
                .filter(expense_events::expense_id.eq_any(exp_ids))
                .load::<ExpenseEvent>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let rexps = expenses
                .into_iter()
                .map(|e| {
                    RenderedExpense::filter_and_render(
                        *uid,
                        e,
                        &transactions[..],
                        &categories,
                        &receipts,
                        &events,
                    )
                })
                .collect::<Vec<_>>();

            Ok(Json(QueryResponse {
                records: rexps,
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

#[get("/expenses/info")]
pub async fn info(uid: UserId, connection: DbConn) -> Result<Json<InfoResponse>, Status> {
    connection
        .run(move |c| {
            let total_record_count = expenses::table
                .select(diesel::dsl::count(expenses::id))
                .filter(
                    expenses::id.eq_any(
                        expenses::table
                            .select(expenses::id)
                            .distinct_on(expenses::id)
                            .left_join(
                                expense_transactions::table
                                    .on(expense_transactions::expense_id.eq(expenses::id)),
                            )
                            .left_join(
                                account_synchronizations::table.on(
                                    expense_transactions::account_id
                                        .eq(account_synchronizations::account1)
                                        .or(expense_transactions::account_id
                                            .eq(account_synchronizations::account2)),
                                ),
                            )
                            .left_join(
                                accounts::table
                                    .on(accounts::id.eq(expense_transactions::account_id)),
                            )
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

            let mut filter_hints = HashMap::<String, Vec<String>>::new();

            let stores = expenses::table
                .select((expenses::store, sql::<BigInt>("count(*) AS cnt")))
                .order(sql::<BigInt>("cnt").desc())
                .filter(
                    expenses::id.eq_any(
                        expenses::table
                            .select(expenses::id)
                            .distinct_on(expenses::id)
                            .left_join(
                                expense_transactions::table
                                    .on(expense_transactions::expense_id.eq(expenses::id)),
                            )
                            .left_join(
                                account_synchronizations::table.on(
                                    expense_transactions::account_id
                                        .eq(account_synchronizations::account1)
                                        .or(expense_transactions::account_id
                                            .eq(account_synchronizations::account2)),
                                ),
                            )
                            .left_join(
                                accounts::table
                                    .on(accounts::id.eq(expense_transactions::account_id)),
                            )
                            .filter(
                                accounts::user_id
                                    .eq(*uid)
                                    .or(account_synchronizations::user1.eq(*uid))
                                    .or(account_synchronizations::user2.eq(*uid)),
                            ),
                    ),
                )
                .filter(expenses::store.ne(""))
                .filter(
                    expenses::booking_end
                        .gt(Utc::now().checked_sub_signed(Duration::weeks(24)).unwrap()),
                )
                .filter(sql("TRUE GROUP BY expenses.store"))
                .limit(25)
                .load::<(String, i64)>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            filter_hints.insert(
                "store".to_owned(),
                stores.into_iter().map(|(s, _)| s).collect::<Vec<_>>(),
            );

            Ok(Json(InfoResponse {
                total_record_count,
                filter_hints,
            }))
        })
        .await
}

#[get("/expenses?<offset>&<count>")]
pub async fn list(
    uid: UserId,
    connection: DbConn,
    offset: Option<i64>,
    count: Option<i64>,
) -> Result<Json<Vec<RenderedExpense>>, Status> {
    connection
        .run(move |c| {
            let exps = queries::relevant_expenses(c, *uid, offset, count)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let transactions =
                queries::expense_transactions_by_expense_range(c, *uid, offset, count)
                    .map_err(|e| log_error_and_500(Box::new(e)))?;
            let categories = queries::expense_categories_by_expense_range(c, *uid, offset, count)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let receipts = queries::expense_receipts_by_expense_range(c, *uid, offset, count)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let events = queries::expense_events_by_expense_range(c, *uid, offset, count)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let mut rexps = Vec::with_capacity(exps.len());

            // all of the above vectors are ordered by expense id, i.e. not the whole vectors have to be searched for every expense.
            let mut t_i = 0;
            let mut c_i = 0;
            let mut r_i = 0;
            let mut e_i = 0;

            for e in exps.into_iter() {
                while transactions
                    .get(t_i)
                    .map(|(t, _, _)| e.id > t.expense_id)
                    .unwrap_or(false)
                {
                    t_i += 1;
                }
                while categories
                    .get(c_i)
                    .map(|(t, _)| e.id > t.expense_id)
                    .unwrap_or(false)
                {
                    c_i += 1;
                }
                while receipts
                    .get(r_i)
                    .map(|t| e.id > t.expense_id)
                    .unwrap_or(false)
                {
                    r_i += 1;
                }
                while events
                    .get(e_i)
                    .map(|t| e.id > t.expense_id)
                    .unwrap_or(false)
                {
                    e_i += 1;
                }

                rexps.push(RenderedExpense::filter_and_render(
                    *uid,
                    e,
                    &transactions[t_i..],
                    &categories[c_i..],
                    &receipts[r_i..],
                    &events[e_i..],
                ));
            }

            // result should be the same, but this is much slower!
            // let rexps = exps
            //     .into_iter()
            //     .map(|e| {
            //         RenderedExpense::filter_and_render(e, &transactions, &categories, &receipts, &events)
            //     })
            //     .collect();

            Ok(Json(rexps))
        })
        .await
}

#[get("/expenses/<id>")]
pub async fn get(
    uid: UserId,
    connection: DbConn,
    id: i32,
) -> Result<Json<RenderedExpense>, Status> {
    connection
        .run(move |c| {
            let exp = queries::relevant_expense_by_id(c, *uid, id)
                .map_err(|e| log_error_and_500(Box::new(e)))?
                .ok_or(Status::NotFound)?;

            let transactions = queries::expense_transactions_by_expense_id(c, *uid, id)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let categories = queries::expense_categories_by_expense_id(c, *uid, id)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let receipts = expense_receipts::table
                .filter(expense_receipts::expense_id.eq(id))
                .load::<ExpenseReceipt>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let events = expense_events::table
                .filter(expense_events::expense_id.eq(id))
                .load::<ExpenseEvent>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            let rexp =
                RenderedExpense::render(*uid, exp, transactions, categories, receipts, events);
            Ok(Json(rexp))
        })
        .await
}

// #[delete("/expenses/<id>")]
// pub fn delete(uid: UserId, connection: DbConn, id: i32) -> Result<(), Status> {
//     todo!() // TODO
// }

// #[put("/expenses/<id>", data = "<balances>")]
// pub fn update(
//     uid: UserId,
//     connection: DbConn,
//     id: i32,
//     account: Json<RenderedBalance>,
// ) -> Result<(), Status> {
//     todo!() // TODO
// }

// #[post("/expenses", data = "<balances>")]
// pub fn create(
//     uid: UserId,
//     connection: DbConn,
//     account: Json<NewBalance>,
// ) -> Result<Json<RenderedBalance>, Status> {
//     todo!() // TODO
// }
