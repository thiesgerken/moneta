use moneta::models::*;
use moneta::schema::*;
use moneta::web::expenses::{SortBy, SortDirection};
use moneta::web::pagination::*;
use moneta::{connect, initialize_logging};

use diesel::prelude::*;
use log::info;
use std::collections::HashMap;

fn main() {
    initialize_logging(2);

    let connection =
        connect("postgres://localhost/moneta").expect("error connecting and migrating to db");

    let request = moneta::web::expenses::QueryRequest {
        from: None,
        to: None,
        needle: None,
        page: 2,
        rows_per_page: 15,
        sort_by: vec![SortBy {
            column: "booking_end".to_owned(),
            direction: SortDirection::Descending,
        }],
        filter_by: [("is_tax_relevant".to_owned(), vec!["true".to_owned()])]
            .iter()
            .cloned()
            .collect(),
    };

    let uid = 1;

    let mut query = expenses::table
        .select(expenses::all_columns)
        .distinct()
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(uid)
                .or(account_synchronizations::user1.eq(uid))
                .or(account_synchronizations::user2.eq(uid)),
        )
        .filter(expenses::is_deleted.eq(false))
        .into_boxed();

    if let Some(from) = request.from {
        query = query.filter(expenses::booking_end.ge(from));
    }
    if let Some(to) = request.to {
        query = query.filter(expenses::booking_start.le(to));
    }

    for sb in request.sort_by.iter() {
        let asc = sb.direction == SortDirection::Ascending;
        // TODO: sorting by amount would be nice

        match sb.column.as_ref() {
            "title" => {
                if asc {
                    query = query.then_order_by(expenses::title.asc());
                } else {
                    query = query.then_order_by(expenses::title.desc());
                }
            }
            "id" => {
                if asc {
                    query = query.then_order_by(expenses::id.asc());
                } else {
                    query = query.then_order_by(expenses::id.desc());
                }
            }
            "store" => {
                if asc {
                    query = query.then_order_by(expenses::store.asc());
                } else {
                    query = query.then_order_by(expenses::store.desc());
                }
            }
            "description" => {
                if asc {
                    query = query.then_order_by(expenses::description.asc());
                } else {
                    query = query.then_order_by(expenses::description.desc());
                }
            }
            "comments" => {
                if asc {
                    query = query.then_order_by(expenses::comments.asc());
                } else {
                    query = query.then_order_by(expenses::comments.desc());
                }
            }
            "bookingStart" | "booking_start" => {
                if asc {
                    query = query.then_order_by(expenses::booking_start.asc());
                } else {
                    query = query.then_order_by(expenses::booking_start.desc());
                }
            }
            "bookingEnd" | "booking_end" => {
                if asc {
                    query = query.then_order_by(expenses::booking_end.asc());
                } else {
                    query = query.then_order_by(expenses::booking_end.desc());
                }
            }
            "isTemplate" | "is_template" => {
                if asc {
                    query = query.then_order_by(expenses::is_template.asc());
                } else {
                    query = query.then_order_by(expenses::is_template.desc());
                }
            }
            "isPreliminary" | "is_preliminary" => {
                if asc {
                    query = query.then_order_by(expenses::is_preliminary.asc());
                } else {
                    query = query.then_order_by(expenses::is_preliminary.desc());
                }
            }
            "isTaxRelevant" | "is_tax_relevant" => {
                if asc {
                    query = query.then_order_by(expenses::is_tax_relevant.asc());
                } else {
                    query = query.then_order_by(expenses::is_tax_relevant.desc());
                }
            }
            _ => {
                // TODO: handle error
            }
        }
    }

    for (column, values) in request.filter_by.iter() {
        match column.as_ref() {
            "title" => {
                query = query.filter(expenses::title.eq_any(values));
            }
            "id" => {
                // TODO: error handling
                let values_parsed = values
                    .into_iter()
                    .map(|s| s.parse().ok())
                    .collect::<Option<Vec<i32>>>()
                    .unwrap();

                query = query.filter(expenses::id.eq_any(values_parsed));
            }
            "store" => {
                query = query.filter(expenses::store.eq_any(values));
            }
            "description" => {
                query = query.filter(expenses::description.eq_any(values));
            }
            "comments" => {
                query = query.filter(expenses::comments.eq_any(values));
            }
            "isTemplate" | "is_template" => {
                // TODO: error handling
                let values_parsed = values
                    .into_iter()
                    .map(|s| s.parse().ok())
                    .collect::<Option<Vec<bool>>>()
                    .unwrap();
                query = query.filter(expenses::is_template.eq_any(values_parsed));
            }
            "isPreliminary" | "is_preliminary" => {
                // TODO: error handling
                let values_parsed = values
                    .into_iter()
                    .map(|s| s.parse().ok())
                    .collect::<Option<Vec<bool>>>()
                    .unwrap();
                query = query.filter(expenses::is_preliminary.eq_any(values_parsed));
            }
            "isTaxRelevant" | "is_tax_relevant" => {
                // TODO: error handling
                let values_parsed = values
                    .into_iter()
                    .map(|s| s.parse().ok())
                    .collect::<Option<Vec<bool>>>()
                    .unwrap();
                query = query.filter(expenses::is_tax_relevant.eq_any(values_parsed));
            }
            _ => {
                // TODO: handle error
            }
        }
    }

    if let Some(s) = request.needle {
        // TODO: more fields to search in (amount would be nice ...)

        let s = s.trim();
        if !s.is_empty() {
            query = query.filter(
                expenses::store
                    .ilike(s.to_owned())
                    .or(expenses::description.ilike(s.to_owned()))
                    .or(expenses::title.ilike(s.to_owned()))
                    .or(expenses::comments.ilike(s.to_owned())),
            );
        }
    }

    let result = query
        .paginate(request.page)
        .per_page(request.rows_per_page)
        .load_and_count::<Expense>(&connection);
    // .map_err(|_| Status::InternalServerError)?;

    info!("{:?}", result);
}
