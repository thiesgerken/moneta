use crate::models::*;
use crate::queries;
use crate::rendering::RenderedExpense;
use crate::schema::*;
use crate::web::user::UserId;
use crate::web::DbConn;

use diesel::prelude::*;
use rocket::http::Status;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

#[get("/expenses?<offset>&<count>")]
pub fn list(
    uid: UserId,
    connection: DbConn,
    offset: Option<i32>,
    count: Option<i32>,
) -> Result<Json<Vec<RenderedExpense>>, Status> {
    let exps = queries::relevant_expenses(&*connection, *uid, offset, count)
        .map_err(|_| Status::InternalServerError)?;
    let transactions =
        queries::expense_transactions_by_expense_range(&*connection, *uid, offset, count)
            .map_err(|_| Status::InternalServerError)?;
    let categories =
        queries::expense_categories_by_expense_range(&*connection, *uid, offset, count)
            .map_err(|_| Status::InternalServerError)?;
    let receipts = queries::expense_receipts_by_expense_range(&*connection, *uid, offset, count)
        .map_err(|_| Status::InternalServerError)?;
    let events = queries::expense_events_by_expense_range(&*connection, *uid, offset, count)
        .map_err(|_| Status::InternalServerError)?;

    let mut rexps = Vec::with_capacity(exps.len());

    // all of the above vectors are ordered by expense id, i.e. not the whole vectors have to be searched for every expense.
    let mut t_i = 0;
    let mut c_i = 0;
    let mut r_i = 0;
    let mut e_i = 0;

    for e in exps.into_iter() {
        while transactions
            .get(t_i)
            .map(|(t, _)| e.id > t.expense_id)
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
}

#[get("/expenses/<id>")]
pub fn get(uid: UserId, connection: DbConn, id: i32) -> Result<Json<RenderedExpense>, Status> {
    let exp = queries::relevant_expense_by_id(&*connection, *uid, id)
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let transactions = queries::expense_transactions_by_expense_id(&*connection, *uid, id)
        .map_err(|_| Status::InternalServerError)?;
    let categories = queries::expense_categories_by_expense_id(&*connection, *uid, id)
        .map_err(|_| Status::InternalServerError)?;
    let receipts = expense_receipts::table
        .filter(expense_receipts::expense_id.eq(id))
        .load::<ExpenseReceipt>(&*connection)
        .map_err(|_| Status::InternalServerError)?;
    let events = expense_events::table
        .filter(expense_events::expense_id.eq(id))
        .load::<ExpenseEvent>(&*connection)
        .map_err(|_| Status::InternalServerError)?;

    let rexp = RenderedExpense::render(exp, transactions, categories, receipts, events);
    Ok(Json(rexp))
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
