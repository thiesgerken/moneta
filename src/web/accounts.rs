use std::str::FromStr;

use crate::models::Account;
use crate::web::authentication::CookieUserId;

use super::util;
use crate::web::util::log_error_and_500;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

#[openapi]
#[get("/accounts/list?<skip>&<limit>&<filter>&<sort>")]
pub async fn list(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
    filter: Option<String>,
    sort: Option<String>,
) -> Result<Json<Vec<Account>>, Status> {
    util::list_query::<Account>("accounts", connection, limit, skip, filter, sort).await
}

#[openapi]
#[get("/accounts/<id>")]
pub async fn get(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<Json<Account>, Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Account>("accounts");
    let entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    Ok(Json(entry))
}

#[openapi]
#[delete("/accounts/<id>")]
pub async fn delete(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<(), Status> {
    // TODO: check that no rendered expenses reference this entry (and that it is not synced)

    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Account>("accounts");
    let entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if *uid != *entry.id {
        return Err(Status::Forbidden);
    }

    collection
        .delete_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(())
}

#[openapi]
#[post("/accounts", data = "<account>")]
pub async fn create(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    account: Json<Account>,
) -> Result<Json<Account>, Status> {
    let mut entry = account.clone();
    entry.id = ObjectId::new().into();
    entry.user_id = (*uid).into();
    entry.synchronization = None;

    let collection = connection.collection::<Account>("accounts");
    collection
        .insert_one(&entry, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(Json(entry))
}

#[openapi]
#[put("/accounts/<id>", data = "<account>")]
pub async fn update(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
    account: Json<Account>,
) -> Result<Json<Account>, Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Account>("accounts");
    let original_entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if *uid != *original_entry.id {
        return Err(Status::Forbidden);
    }

    // TODO: need to update the CLI for CRUD on account synchronizations

    let mut entry = account.clone();
    entry.id = original_entry.id;
    entry.user_id = original_entry.user_id;
    entry.synchronization = original_entry.synchronization;

    collection
        .update_one(
            doc! {"_id":id},
            doc! {"$set":  bson::to_bson(&entry).unwrap()},
            None,
        )
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(Json(entry))
}
