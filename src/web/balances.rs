use std::str::FromStr;

use crate::models::rendering::RenderedBalance;
use crate::models::Balance;
use crate::queries;
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
#[get("/balances/<upstream_id>")]
pub async fn get(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    upstream_id: String,
) -> Result<Json<RenderedBalance>, Status> {
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedBalance>("balances_rendered");
    let entry = collection
        .find_one(doc! {"upstream_id":upstream_id, "user_id": *uid}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if *entry.user_id != *uid {
        return Err(Status::Forbidden);
    }

    Ok(Json(entry))
}

#[openapi]
#[delete("/balances/<upstream_id>")]
pub async fn delete(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    upstream_id: String, // id of the upstream balance
) -> Result<(), Status> {
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedBalance>("balances_rendered");
    // check if there is a rendered balance with the given upstream id belonging to this user
    collection
        .find_one(doc! {"upstream_id":upstream_id, "user_id": *uid}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    let upstream_collection = connection.collection::<Balance>("balances");
    let upstream_entry = upstream_collection
        .find_one(doc! {"_id":upstream_id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    // remove rendered balances first ...
    upstream_entry
        .remove_rendered(connection)
        .await
        .map_err(log_error_and_500)?;
    // ... and then the upstream balance itself
    upstream_collection
        .delete_one(doc! {"_id":upstream_id}, None)
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
    let related_accounts = queries::query_associated_accounts(connection, &[*balance.account_id])
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    let account = related_accounts
        .iter()
        .find(|a| a.id == balance.account_id)
        .ok_or_else(|| {
            error!(
                "user {:?} tried to create a balance with unknown account {:?}",
                *uid, &balance.account_id
            );
            Status::NotFound
        })?;

    if *account.user_id != *uid {
        error!(
            "user {:?} tried to create a balance with foreign account {:?}",
            *uid, &balance.account_id
        );
        return Err(Status::Forbidden);
    }

    // cast to normal balance and insert into database
    let entry = Balance::from_rendered(balance.clone());
    let collection = connection.collection::<Balance>("balances");
    collection
        .insert_one(&entry, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // create rendered balances and insert them into database
    let rendered_balances = entry.render(&related_accounts).map_err(log_error_and_500)?;
    let rendered_balances_collection =
        connection.collection::<RenderedBalance>("balances_rendered");
    rendered_balances_collection
        .insert_many(&rendered_balances, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // find the rendered balance that was rendered for this user and return it
    let entry = rendered_balances
        .into_iter()
        .find(|b| *b.user_id == *uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a balance for this user?! balance {:?}",
                entry.id
            )
        });
    Ok(Json(entry))
}

#[openapi]
#[put("/balances/<upstream_id>", data = "<balance>")]
pub async fn update(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    upstream_id: String, // id of the upstream balance
    balance: Json<RenderedBalance>,
) -> Result<Json<RenderedBalance>, Status> {
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedBalance>("balances_rendered");
    let original_entry = collection
        .find_one(doc! {"upstream_id":upstream_id, "user_id": *uid}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    let related_accounts = queries::query_associated_accounts(connection, &[*balance.account_id])
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    let account = related_accounts
        .iter()
        .find(|a| a.id == balance.account_id)
        .ok_or_else(|| {
            error!(
                "user {:?} tried to create a balance with unknown account {:?}",
                *uid, &balance.account_id
            );
            Status::NotFound
        })?;

    if *account.user_id != *uid {
        error!(
            "user {:?} tried to create a balance with foreign account {:?}",
            *uid, &balance.account_id
        );
        return Err(Status::Forbidden);
    }

    // find the original upstream balance
    let upstream_collection = connection.collection::<Balance>("balances");
    let mut upstream_entry = upstream_collection
        .find_one(doc!("_id": *original_entry.upstream_id), None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .unwrap_or_else(|| {
            panic!(
                "cannot find upstream balance for rendered balance {:?}",
                original_entry.id
            )
        });

    // delete its rendered balances
    upstream_entry
        .remove_rendered(connection)
        .await
        .map_err(log_error_and_500)?;

    // update the upstream balance with the changes
    upstream_entry.apply_changes(&*balance);
    upstream_collection
        .update_one(
            doc!("_id": *upstream_entry.id),
            doc! {"$set":  bson::to_bson(&upstream_entry).unwrap()},
            None,
        )
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // create rendered balances and insert them into database
    let rendered_balances = upstream_entry
        .render(&related_accounts)
        .map_err(log_error_and_500)?;
    collection
        .insert_many(&rendered_balances, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // find the rendered balance that was rendered for this user and return it
    let entry = rendered_balances
        .into_iter()
        .find(|b| *b.user_id == *uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a balance for this user?! balance {:?}",
                upstream_entry.id
            )
        });
    Ok(Json(entry))
}
