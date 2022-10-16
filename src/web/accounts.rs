use std::str::FromStr;

use crate::models::rendering::{RenderedBalance, RenderedExpense};
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
#[delete("/accounts/<id>")]
pub async fn delete(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
) -> Result<(), Status> {
    let id = ObjectId::from_str(&id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<Account>("accounts");
    let entry = collection
        .find_one(doc! {"_id":id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if entry.synchronization.is_some() {
        error!(
            "Deletion of synchronized accounts is unsupported, tried to remove {:?}",
            id
        );
        return Err(Status::Forbidden);
    }

    if *uid != *entry.user_id {
        return Err(Status::Forbidden);
    }

    //  check that no rendered balances reference this entry
    let rendered_balances_collection =
        connection.collection::<RenderedBalance>("balances_rendered");
    let balance_count = rendered_balances_collection
        .count_documents(doc!("account_id": *entry.id.clone()), None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    if balance_count > 0 {
        error!(
            "Tried to remove account {:?}, but it still has {} (rendered) balances attached to it",
            id, balance_count
        );
        return Err(Status::Forbidden);
    }

    // check that no rendered expenses reference this entry
    let rendered_expenses_collection =
        connection.collection::<RenderedExpense>("expenses_rendered");
    let expense_count = rendered_expenses_collection
        .count_documents(doc!("account_id": *entry.id.clone()), None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    if expense_count > 0 {
        error!(
            "Tried to remove account {:?}, but it still has {} (rendered) expenses attached to it",
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
#[post("/accounts", data = "<account>")]
pub async fn create(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    account: Json<Account>,
) -> Result<Json<Account>, Status> {
    let mut entry = account;
    entry.id = ObjectId::new().into();
    entry.user_id = (*uid).into();
    entry.validate().map_err(|e| log_error_and_500(e))?;

    if entry.synchronization.is_some() {
        error!("Creation of synchronized accounts is unsupported");
        return Err(Status::Forbidden);
    }

    let collection = connection.collection::<Account>("accounts");
    collection
        .insert_one(&*entry, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(entry)
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

    if *uid != *original_entry.user_id {
        return Err(Status::Forbidden);
    }

    if account.synchronization != original_entry.synchronization {
        error!("Modification of account synchronizations is unsupported");
        return Err(Status::Forbidden);
    }

    let mut entry = account.0.clone();
    entry.id = original_entry.id;
    entry.user_id = original_entry.user_id;
    entry.synchronization = original_entry.synchronization;
    entry.validate().map_err(|e| log_error_and_500(e))?;

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
