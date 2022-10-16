use log::error;
use rocket::http::Status;
use std::error::Error;

use futures::{StreamExt, TryStreamExt};
use mongodb::bson::{self, Document};
use mongodb::options::FindOptions;
use rocket::serde::json::Json;
use serde::de::DeserializeOwned;

pub fn log_error_and_500(e: Box<dyn Error>) -> Status {
    error!("{}", e);
    Status::InternalServerError
}

pub fn string_to_document(s: &str) -> Result<Document, Box<dyn Error>> {
    let x: serde_json::Value = serde_json::from_str(s)?;
    bson::to_document(&x).map_err(|e| e.into())
}

pub async fn list_query<T>(
    collection_name: &str,
    connection: &mongodb::Database,
    limit: Option<i32>,
    skip: Option<i32>,
    filter: Option<String>,
    sort: Option<String>,
    filter_overrides: Option<Document>,
) -> Result<Vec<T>, Status>
where
    T: DeserializeOwned + Unpin + Send + Sync,
{
    let filter = filter
        .map(|f| string_to_document(&f))
        .transpose()
        .map_err(|e| {
            error!("Unable to parse filtering options: {}", e);
            Status::UnprocessableEntity
        })?;

    let filter = if let Some(ov) = filter_overrides {
        let mut f = filter.unwrap_or_default();
        f.extend(ov.into_iter());
        Some(f)
    } else {
        filter
    };

    let sort = sort
        .map(|s| string_to_document(&s))
        .transpose()
        .map_err(|e| {
            error!("Unable to parse sorting options: {}", e);
            Status::UnprocessableEntity
        })?;
    let find_options = sort.map(|s| FindOptions::builder().sort(s).build());

    let collection = connection.collection::<T>(collection_name);
    let cursor = collection
        .find(filter, find_options)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .skip(usize::try_from(skip.unwrap_or(0)).map_err(|e| log_error_and_500(Box::new(e)))?)
        .take(
            usize::try_from(limit.unwrap_or(i32::MAX))
                .map_err(|e| log_error_and_500(Box::new(e)))?,
        );

    let entries = cursor
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(entries)
}
