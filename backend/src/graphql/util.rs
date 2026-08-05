use std::error::Error;

use futures::{StreamExt, TryStreamExt};
use mongodb::options::FindOptions;
use serde::de::DeserializeOwned;
use similar::TextDiff;
use uuid::Uuid;

pub fn string_to_document(s: &str) -> Result<Document, Box<dyn Error>> {
    // also handles EJSON (i.e. decodes {$oid: "..."} as Uuid if possible)
    let x: serde_json::Value = serde_json::from_str(s)?;
    bson::serialize_to_document(&x).map_err(|e| e.into())
}

/// Merges a caller-supplied filter with the overrides that scope a query to its user.
///
/// The overrides are applied last on purpose: they carry the `userId` restriction, so a
/// caller must not be able to widen their own scope by naming the same key in `filter`.
pub fn merge_filter(
    filter: Option<String>,
    filter_overrides: Option<Document>,
) -> Result<Document, Box<dyn Error>> {
    let filter = filter
        .map(|f| string_to_document(&f))
        .transpose()
        .map_err(|e| format!("Unable to parse filtering options: {}", e))?;

    let mut filter = filter.unwrap_or_default();
    if let Some(ov) = filter_overrides {
        filter.extend(ov);
    }

    Ok(filter)
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
    let filter = merge_filter(filter, filter_overrides)?;

    let sort = sort
        .map(|s| string_to_document(&s))
        .transpose()
        .map_err(|e| format!("Unable to parse sorting options: {}", e))?;
    let find_options = sort.map(|s| FindOptions::builder().sort(s).build());

    let collection = connection.collection::<T>(collection_name);
    let total_count = collection.count_documents(filter.clone()).await?;
    let cursor = collection
        .find(filter)
        .with_options(find_options)
        .await?
        .skip(usize::try_from(skip.unwrap_or(0))?)
        .take(usize::try_from(limit.unwrap_or(i32::MAX))?);

    let edges = cursor.try_collect::<Vec<_>>().await?;

    Ok((total_count, edges))
}

pub fn json_diff<T: serde::Serialize>(
    left: &T,
    right: &T,
) -> Result<String, Box<dyn std::error::Error>> {
    let left_json = serde_json::to_string_pretty(left)? + "\n";
    let right_json = serde_json::to_string_pretty(right)? + "\n";

    let diff = TextDiff::from_lines(&left_json, &right_json);
    Ok(diff.unified_diff().context_radius(3).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_filters() {
        let d = string_to_document(r#"{"title": "Rewe", "isTemplate": false}"#).unwrap();
        assert_eq!(d.get_str("title").unwrap(), "Rewe");
        assert!(!d.get_bool("isTemplate").unwrap());
    }

    /// the frontend sends account and category ids as extended json
    #[test]
    fn decodes_extended_json_object_ids() {
        let oid = Uuid::now_v7();
        let d = string_to_document(&format!(
            r#"{{"accountId": {{"$oid": "{}"}}}}"#,
            oid.to_hex()
        ))
        .unwrap();

        assert_eq!(d.get_object_id("accountId").unwrap(), oid);
    }

    #[test]
    fn rejects_malformed_filters() {
        assert!(string_to_document("not json").is_err());
        assert!(string_to_document(r#"{"unterminated": "#).is_err());
    }

    #[test]
    fn merges_filter_with_overrides() {
        let uid = Uuid::now_v7();
        let merged = merge_filter(
            Some(r#"{"store": "Rewe"}"#.into()),
            Some(doc!("userId": uid)),
        )
        .unwrap();

        assert_eq!(merged.get_str("store").unwrap(), "Rewe");
        assert_eq!(merged.get_object_id("userId").unwrap(), uid);
    }

    /// the override carries the authorization scope, so a caller naming `userId`
    /// themselves must not be able to read another user's entries
    #[test]
    fn caller_cannot_override_the_user_scope() {
        let own = Uuid::now_v7();
        let someone_else = Uuid::now_v7();

        let merged = merge_filter(
            Some(format!(
                r#"{{"userId": {{"$oid": "{}"}}}}"#,
                someone_else.to_hex()
            )),
            Some(doc!("userId": own)),
        )
        .unwrap();

        assert_eq!(merged.get_object_id("userId").unwrap(), own);
    }

    #[test]
    fn keeps_operator_filters_alongside_the_user_scope() {
        let uid = Uuid::now_v7();
        let merged = merge_filter(
            Some(r#"{"$or": [{"store": "Rewe"}, {"store": "Aldi"}]}"#.into()),
            Some(doc!("userId": uid)),
        )
        .unwrap();

        // both survive, so mongo ANDs them and the scope still applies
        assert!(matches!(merged.get("$or"), Some(Bson::Array(a)) if a.len() == 2));
        assert_eq!(merged.get_object_id("userId").unwrap(), uid);
    }

    #[test]
    fn empty_filter_still_scopes_to_the_user() {
        let uid = Uuid::now_v7();
        let merged = merge_filter(None, Some(doc!("userId": uid))).unwrap();

        assert_eq!(merged.len(), 1);
        assert_eq!(merged.get_object_id("userId").unwrap(), uid);
    }

    #[test]
    fn propagates_parse_errors() {
        assert!(merge_filter(Some("{{{".into()), None).is_err());
    }
}
