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

#[get("/balances?<offset>&<count>")]
pub fn list(
    uid: UserId,
    connection: DbConn,
    offset: Option<i32>,
    count: Option<i32>,
) -> Option<Json<Vec<RenderedBalance>>> {
    let accs = queries::relevant_balances(&*connection, *uid, offset, count)
        .ok()?
        .into_iter()
        .map(|(balance, syncing)| RenderedBalance::render(balance, syncing))
        .collect();

    Some(Json(accs))
}

#[get("/balances/<id>")]
pub fn get(uid: UserId, connection: DbConn, id: i32) -> Option<Json<RenderedBalance>> {
    let (balance, syncing) = queries::relevant_balance_by_id(&*connection, *uid, id).ok()??;

    Some(Json(RenderedBalance::render(balance, syncing)))
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
