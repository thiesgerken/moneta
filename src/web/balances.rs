use std::str::FromStr;

use crate::models::rendering::RenderedBalance;
use crate::models::Balance;
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
#[get("/balances/list?<skip>&<limit>&<filter>&<sort>")]
pub async fn list(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
    filter: Option<String>,
    sort: Option<String>,
) -> Result<Json<Vec<RenderedBalance>>, Status> {
    util::list_query::<RenderedBalance>(
        "balances_rendered",
        connection,
        limit,
        skip,
        filter,
        sort,
        Some(doc!("user_id": *uid)), // need to make sure that the balance was rendered for this user
    )
    .await
}

#[openapi]
#[get("/balances/<id>")]
pub async fn get(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<Json<RenderedBalance>, Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedBalance>("balances_rendered");
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
#[delete("/balances/<id>")]
pub async fn delete(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String, // id of the upstream balance
) -> Result<(), Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedBalance>("balances_rendered");
    let entry = collection
        .find_one(doc! {"upstream_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if *uid != *entry.user_id {
        return Err(Status::Forbidden);
    }

    let upstream_collection = connection.collection::<Balance>("balances");
    let upstream_entry = upstream_collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    // remove rendered balances first ...
    upstream_entry
        .remove_rendered(&connection)
        .await
        .map_err(log_error_and_500)?;
    // ... and then the upstream balance itself
    upstream_collection
        .delete_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(())
}

#[openapi]
#[post("/balances", data = "<balance>")]
pub async fn create(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    balance: Json<RenderedBalance>,
) -> Result<Json<RenderedBalance>, Status> {
    todo!() // TODO: creation of rendered balances
}

#[openapi]
#[put("/balances/<id>", data = "<balance>")]
pub async fn update(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
    balance: Json<RenderedBalance>,
) -> Result<Json<RenderedBalance>, Status> {
    todo!() // TODO: updating of rendered balances
}
