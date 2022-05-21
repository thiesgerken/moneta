use std::str::FromStr;

use crate::models::rendering::RenderedExpense;
use crate::models::{Expense, Category};
use crate::queries;
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
        .find_one(doc! {"upstream_id":upstream_id, "user_id": *uid}, None)
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

    if occurring_account_ids
        .iter()
        .any(|a_id| related_accounts.iter().find(|a| *a.id == *a_id).is_none())
    {
        error!(
            "user {:?} tried to create an expense with unknown account id(s)",
            *uid
        );
        return Err(Status::NotFound);
    };

    if related_accounts.iter().all(|a| *a.user_id != *uid) {
        error!(
                "user {:?} tried to create an expense consisting only of accounts that don't belong to her",
                *uid
            );
        return Err(Status::Forbidden);
    }

    let category_collection = connection.collection::<Category>("categories");
    let categories = category_collection
        .find(doc!(), None) // can't be that many, right?
        .await?
        .try_collect::<Vec<_>>()
        .await?;
        if occurring_account_ids
        .iter()
        .any(|a_id| related_accounts.iter().find(|a| *a.id == *a_id).is_none())
    {
        error!(
            "user {:?} tried to create an expense with unknown category id(s)",
            *uid
        );
        return Err(Status::NotFound);
    };

    // cast to normal expense and insert into database
    let entry = Expense::from_rendered(expense.clone());
    entry.validate()  .map_err(log_error_and_500)?;

    let collection = connection.collection::<Expense>("expenses");
    collection
        .insert_one(&entry, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // create rendered expenses and insert them into database
    let rendered_expenses = entry.render(&related_accounts, &categories).map_err(log_error_and_500)?;
    let rendered_expenses_collection =
        connection.collection::<RenderedExpense>("expenses_rendered");
    rendered_expenses_collection
        .insert_many(&rendered_expenses, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // find the rendered expense that was rendered for this user and return it
    let entry = rendered_expenses
        .into_iter()
        .find(|b| *b.user_id == *uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a expense for this user?! expense {:?}",
                entry.id
            )
        });
    Ok(Json(entry))

    // TODO: creation of rendered expenses
}

#[openapi]
#[put("/expenses/<id>", data = "<expense>")]
pub async fn update(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
    id: String,
    expense: Json<RenderedExpense>,
) -> Result<Json<RenderedExpense>, Status> {
    let upstream_id =
        ObjectId::from_str(&upstream_id).map_err(|e| log_error_and_500(Box::new(e)))?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    let original_entry = collection
        .find_one(doc! {"upstream_id":upstream_id, "user_id": *uid}, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::NotFound)?;

    let related_accounts = queries::query_associated_accounts(connection, &[*expense.account_id])
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    let account = related_accounts
        .iter()
        .find(|a| a.id == expense.account_id)
        .ok_or_else(|| {
            error!(
                "user {:?} tried to create a expense with unknown account {:?}",
                *uid, &expense.account_id
            );
            Status::NotFound
        })?;

    if *account.user_id != *uid {
        error!(
            "user {:?} tried to create a expense with foreign account {:?}",
            *uid, &expense.account_id
        );
        return Err(Status::Forbidden);
    }

    // find the original upstream expense
    let upstream_collection = connection.collection::<Expense>("expenses");
    let mut upstream_entry = upstream_collection
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
    upstream_entry.apply_changes(&*expense);
    upstream_collection
        .update_one(
            doc!("_id": *upstream_entry.id),
            doc! {"$set":  bson::to_bson(&upstream_entry).unwrap()},
            None,
        )
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // create rendered expenses and insert them into database
    let rendered_expenses = upstream_entry
        .render(&related_accounts)
        .map_err(log_error_and_500)?;
    collection
        .insert_many(&rendered_expenses, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    // find the rendered expense that was rendered for this user and return it
    let entry = rendered_expenses
        .into_iter()
        .find(|b| *b.user_id == *uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a expense for this user?! expense {:?}",
                upstream_entry.id
            )
        });
    Ok(Json(entry))

    // TODO: updating of rendered expenses
}

// TODO: implement changelogs for expenses
// TODO: keep in mind to validate expenses
