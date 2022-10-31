use futures::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId};
use serde::de::DeserializeOwned;

use crate::models::{Account, User};

pub mod optional_chrono_datetime_as_bson_datetime {
    use chrono::Utc;
    use mongodb::bson;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes a [`Option<chrono::DateTime>`] from a [`Option<crate::DateTime>`].
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<chrono::DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Ok(datetime) = bson::DateTime::deserialize(deserializer) {
            Ok(Some(datetime.to_chrono()))
        } else {
            // NOTE: this implementation simply returns `None` whenever the value present
            // cannot be parsed as a datetime; this might not be the best behavior
            Ok(None)
        }
    }

    /// Serializes a [`Option<chrono::DateTime>`] as a [`Option<crate::DateTime>`].
    pub fn serialize<S: Serializer>(
        val: &Option<chrono::DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if let Some(val) = val {
            let datetime = bson::DateTime::from_chrono(val.to_owned());
            datetime.serialize(serializer)
        } else {
            bson::Bson::Null.serialize(serializer)
        }
    }
}

pub async fn query_element_by_id<T>(
    connection: &mongodb::Database,
    collection: &str,
    id: ObjectId,
) -> Result<Option<T>, mongodb::error::Error>
where
    T: DeserializeOwned + Unpin + Send + Sync,
{
    let collection = connection.collection::<T>(collection);
    let filter = doc! {"_id":id};
    collection.find_one(filter, None).await
}

pub async fn query_associated_accounts(
    connection: &mongodb::Database,
    account_ids: &[ObjectId],
) -> Result<Vec<Account>, mongodb::error::Error> {
    let collection = connection.collection::<Account>("accounts");

    collection.find(doc! { "$or": [ {"_id": {"$in": account_ids}}, {"synchronization.accountId": {"$in": account_ids}}] }, None).await?   .try_collect::<Vec<_>>()
    .await
}

pub async fn query_user_by_name(
    connection: &mongodb::Database,
    user_name: &str,
) -> Result<Option<User>, mongodb::error::Error> {
    let collection = connection.collection::<User>("users");
    let filter = doc! {"name":user_name};
    collection.find_one(filter, None).await
}
