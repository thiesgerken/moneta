use crate::queries;
use crate::rendering::RenderedBalance;
use crate::web::user::UserId;
use crate::web::DbConn;

use rocket::http::Status;
use rocket_contrib::json::Json;

#[get("/balances?<offset>&<count>")]
pub fn list(
    uid: UserId,
    connection: DbConn,
    offset: Option<i32>,
    count: Option<i32>,
) -> Result<Json<Vec<RenderedBalance>>, Status> {
    let bals = queries::relevant_balances(&*connection, *uid, offset, count)
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|(balance, syncing)| RenderedBalance::render(balance, syncing))
        .collect();

    Ok(Json(bals))
}

#[get("/balances/<id>")]
pub fn get(uid: UserId, connection: DbConn, id: i32) -> Result<Json<RenderedBalance>, Status> {
    let (balance, syncing) = queries::relevant_balance_by_id(&*connection, *uid, id)
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    Ok(Json(RenderedBalance::render(balance, syncing)))
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
