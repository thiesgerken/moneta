use std::error::Error;

use futures::{StreamExt, TryStreamExt};
use mongodb::bson::{self, Document};
use mongodb::options::FindOptions;
use serde::de::DeserializeOwned;

pub fn string_to_document(s: &str) -> Result<Document, Box<dyn Error>> {
    // also handles EJSON (i.e. decodes {$oid: "..."} as ObjectId if possible)
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
) -> Result<(u64, Vec<T>), Box<dyn Error>>
where
    T: DeserializeOwned + Unpin + Send + Sync,
{
    let filter = filter
        .map(|f| string_to_document(&f))
        .transpose()
        .map_err(|e| format!("Unable to parse filtering options: {}", e))?;

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
        .map_err(|e| format!("Unable to parse sorting options: {}", e))?;
    let find_options = sort.map(|s| FindOptions::builder().sort(s).build());

    let collection = connection.collection::<T>(collection_name);
    let total_count = collection.count_documents(filter.clone(), None).await?;
    let cursor = collection
        .find(filter, find_options)
        .await?
        .skip(usize::try_from(skip.unwrap_or(0))?)
        .take(usize::try_from(limit.unwrap_or(i32::MAX))?);

    let edges = cursor.try_collect::<Vec<_>>().await?;

    Ok((total_count, edges))
}
