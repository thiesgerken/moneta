use crate::models::*;
use crate::queries;
use crate::rendering::RenderedBalance;
use crate::schema::*;
use crate::web::user::UserId;
use crate::web::DbConn;

use diesel::prelude::*;
use log::info;
use rocket::http::Status;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

#[get("/expenses?<offset>&<count>")]
pub fn list(
    uid: UserId,
    connection: DbConn,
    offset: Option<i32>,
    count: Option<i32>,
) -> Option<Json<Vec<Expense>>> {
    let exps = queries::relevant_expenses(&*connection, *uid, offset, count).ok()?;

    // TODO: change to RenderedExpense
    // TODO: query the rest of *needed* data (transactions: also get account sync for the account!)

    Some(Json(exps))
}

#[get("/expenses/<id>")]
pub fn get(uid: UserId, connection: DbConn, id: i32) -> Option<Json<Expense>> {
    let exp = queries::relevant_expense_by_id(&*connection, *uid, id).ok()??;

    // TODO: change to RenderedExpense
    // TODO: query the rest of *needed* data (transactions: also get account sync for the account!)

    Some(Json(exp))
    // Some(Json(RenderedBalance::render(balance, syncing)))
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
