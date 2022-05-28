use std::str::FromStr;

use crate::models::rendering::RenderedExpense;
use crate::models::{Account, Expense};
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

pub async fn validate_and_unrender(
    uid: &CookieUserId,
    expense: Json<RenderedExpense>,
    related_accounts: &[Account],
) -> Result<Expense, Status> {
    if expense
        .transactions
        .iter()
        .map(|t| *t.account_id)
        .any(|a_id| !related_accounts.iter().any(|a| *a.id == a_id))
    {
        error!(
            "user {:?} tried to create an expense with unknown account id(s)",
            **uid
        );
        return Err(Status::NotFound);
    };

    if related_accounts.iter().all(|a| *a.user_id != **uid) {
        error!(
                "user {:?} tried to create an expense consisting only of accounts that don't belong to her",
                **uid
            );
        return Err(Status::Forbidden);
    }

    if expense
        .categories
        .iter()
        .map(|c| c.category_id.clone())
        .any(|a_id| !related_accounts.iter().any(|a| *a.id == *a_id))
    {
        error!(
            "user {:?} tried to create an expense with unknown category id(s)",
            **uid
        );
        return Err(Status::NotFound);
    };

    // cast to normal expense and insert into database
    let entry = Expense::from_rendered(expense.clone());
    entry.validate().map_err(log_error_and_500)?;

    Ok(entry)
}

pub async fn render(
    uid: &CookieUserId,
    connection: &State<mongodb::Database>,
    entry: Expense,
    related_accounts: &[Account],
) -> Result<Json<RenderedExpense>, Status> {
    // create rendered expenses and insert them into database
    let rendered_expenses = entry.render(related_accounts).map_err(log_error_and_500)?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    collection
        .insert_many(&rendered_expenses, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // find the rendered expense that was rendered for this user and return it
    let rendered_entry = rendered_expenses
        .into_iter()
        .find(|b| *b.user_id == **uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a expense for this user?! expense {:?}",
                entry.id
            )
        });
    Ok(Json(rendered_entry))
}

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
        Some(doc!("userId": *uid)), // need to make sure that the expense was rendered for this user
    )
    .await
}

#[openapi]
#[get("/expenses/<upstream_id>")]
pub async fn get(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    upstream_id: String,
) -> Result<Json<RenderedExpense>, Status> {
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    let entry = collection
        .find_one(doc! {"_id":upstream_id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    if *entry.user_id != *uid {
        return Err(Status::Forbidden);
    }

    Ok(Json(entry))
}

#[openapi]
#[delete("/expenses/<upstream_id>")]
pub async fn delete(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    upstream_id: String,
) -> Result<(), Status> {
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    // check if there is a rendered expense with the given upstream id belonging to this user
    collection
        .find_one(doc! {"upstreamId":upstream_id, "userId": *uid}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    let upstream_collection = connection.collection::<Expense>("expenses");
    let upstream_entry = upstream_collection
        .find_one(doc! {"_id":upstream_id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    // remove rendered expenses first ...
    upstream_entry
        .remove_rendered(connection)
        .await
        .map_err(log_error_and_500)?;
    // ... and then the upstream expense itself
    upstream_collection
        .delete_one(doc! {"_id":upstream_id}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(())
}

#[openapi]
#[post("/expenses", data = "<expense>")]
pub async fn create(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    expense: Json<RenderedExpense>,
) -> Result<Json<RenderedExpense>, Status> {
    let occurring_account_ids: Vec<_> =
        expense.transactions.iter().map(|t| *t.account_id).collect();
    let related_accounts = queries::query_associated_accounts(connection, &occurring_account_ids)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    let entry = validate_and_unrender(&uid, expense, &related_accounts).await?;

    let collection = connection.collection::<Expense>("expenses");
    collection
        .insert_one(&entry, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    render(&uid, connection, entry, &related_accounts).await
}

#[openapi]
#[put("/expenses/<upstream_id>", data = "<expense>")]
pub async fn update(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    upstream_id: String, // id of the upstream expense
    expense: Json<RenderedExpense>,
) -> Result<Json<RenderedExpense>, Status> {
    let occurring_account_ids: Vec<_> =
        expense.transactions.iter().map(|t| *t.account_id).collect();
    let related_accounts = queries::query_associated_accounts(connection, &occurring_account_ids)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    let mut entry = validate_and_unrender(&uid, expense, &related_accounts).await?;

    // check if the original rendered expense belongs to this user
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    let original_entry = collection
        .find_one(doc! {"upstreamId":upstream_id, "userId": *uid}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    // find the original upstream expense
    let upstream_collection = connection.collection::<Expense>("expenses");
    let upstream_entry = upstream_collection
        .find_one(doc!("_id": *original_entry.upstream_id), None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .unwrap_or_else(|| {
            panic!(
                "cannot find upstream expense for rendered expense {:?}",
                original_entry.id
            )
        });

    // delete its rendered expenses
    upstream_entry
        .remove_rendered(connection)
        .await
        .map_err(log_error_and_500)?;

    // update the upstream expense with the changes
    entry.id = upstream_entry.id;
    upstream_collection
        .update_one(
            doc!("_id": &*entry.id),
            doc! {"$set":  bson::to_bson(&entry).unwrap()},
            None,
        )
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    render(&uid, connection, entry, &related_accounts).await
}

// TODO: implement changelogs for expenses
