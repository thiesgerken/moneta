use std::str::FromStr;

use crate::models::rendering::RenderedExpense;
use crate::web::authentication::CookieUserId;

use super::util;
use crate::web::util::log_error_and_500;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

#[openapi]
#[get("/expenses/list?<skip>&<limit>&<filter>&<sort>")]
pub async fn list(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
    filter: Option<String>,
    sort: Option<String>,
) -> Result<Json<Vec<RenderedExpense>>, Status> {
    util::list_query::<RenderedExpense>(
        "expenses_rendered",
        connection,
        limit,
        skip,
        filter,
        sort,
        Some(doc!("user_id": *uid)), // need to make sure that the expense was rendered for this user
    )
    .await
}

#[openapi]
#[get("/expenses/<id>")]
pub async fn get(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<Json<RenderedExpense>, Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    let entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if *entry.user_id != *uid {
        return Err(Status::Forbidden);
    }

    Ok(Json(entry))
}

#[openapi]
#[delete("/expenses/<id>")]
pub async fn delete(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<(), Status> {
    todo!() // TODO: deletion of rendered expenses
}

#[openapi]
#[post("/expenses", data = "<expense>")]
pub async fn create(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    expense: Json<RenderedExpense>,
) -> Result<Json<RenderedExpense>, Status> {
    todo!() // TODO: creation of rendered expenses
}

#[openapi]
#[put("/expenses/<id>", data = "<expense>")]
pub async fn update(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
    expense: Json<RenderedExpense>,
) -> Result<Json<RenderedExpense>, Status> {
    todo!() // TODO: updating of rendered expenses
}

// TODO: implement changelogs for expenses
// TODO: keep in mind to validate expenses
