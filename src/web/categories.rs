use std::str::FromStr;

use crate::models::rendering::RenderedExpense;
use crate::models::Category;
use crate::web::authentication::CookieUserId;

use crate::web::util::log_error_and_500;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

use super::util;

#[openapi]
#[get("/categories/list?<skip>&<limit>&<filter>&<sort>")]
pub async fn list(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
    filter: Option<String>,
    sort: Option<String>,
) -> Result<Json<Vec<Category>>, Status> {
    util::list_query::<Category>("categories", connection, limit, skip, filter, sort, None).await
}

#[openapi]
#[get("/categories/<id>")]
pub async fn get(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<Json<Category>, Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Category>("categories");
    let entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    Ok(Json(entry))
}

#[openapi]
#[delete("/categories/<id>")]
pub async fn delete(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<(), Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Category>("categories");
    let entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    // check that no rendered expenses reference this entry
    let rendered_expenses_collection =
        connection.collection::<RenderedExpense>("expenses_rendered");
    let expense_count = rendered_expenses_collection
        .count_documents(doc!("categories.category_id": *entry.id.clone()), None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    if expense_count > 0 {
        error!(
            "Tried to remove category {:?}, but it still has {} (rendered) expenses attached to it",
            id, expense_count
        );
        return Err(Status::Forbidden);
    }

    // everything seems to be in order, proceed with the deletion
    collection
        .delete_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(())
}

#[openapi]
#[post("/categories", data = "<category>")]
pub async fn create(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    category: Json<Category>,
) -> Result<Json<Category>, Status> {
    let mut entry = category.clone();
    entry.id = ObjectId::new().into();

    let collection = connection.collection::<Category>("categories");
    collection
        .insert_one(&entry, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(Json(entry))
}

#[openapi]
#[put("/categories/<id>", data = "<category>")]
pub async fn update(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
    category: Json<Category>,
) -> Result<Json<Category>, Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Category>("categories");
    let original_entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    let mut entry = category.clone();
    entry.id = original_entry.id;

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
