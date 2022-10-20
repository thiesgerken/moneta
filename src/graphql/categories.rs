use crate::models::rendering::RenderedExpense;
use crate::models::{Category, CategoryCreateInput, CategoryUpdateInput, ObjectIdWrapper};
use crate::graphql::root::require_user;
use crate::graphql::root::Context;
use crate::graphql::util;

use juniper::graphql_object;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};

pub struct CategoryQuery;

#[graphql_object(
    Context = Context,
)]
impl CategoryQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<Category>> {
        require_user(context).await?;

        Ok(util::list_query::<Category>(
            "categories",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            None,
        )
        .await?)
    }

    pub async fn get(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<Category> {
        require_user(context).await?;
        let collection = context.connection.collection::<Category>("categories");

        let entry = collection.find_one(doc! {"_id":*id}, None).await?;

        if let Some(entry) = entry {
            Ok(entry)
        } else {
            Err(juniper::FieldError::new(
                "Category does not exist",
                juniper::Value::null(),
            ))
        }
    }
}

pub struct CategoryMutation;

#[graphql_object(
    Context = Context,
)]
impl CategoryMutation {
    pub async fn delete(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<bool> {
        require_user(context).await?;
        let collection = context.connection.collection::<Category>("categories");

        let category = collection
            .find_one(doc! {"_id":*id}, None)
            .await?
            .ok_or("entry to delete not found")?;

        // check that no rendered expenses reference this entry
        let rendered_expenses_collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");
        let expense_count = rendered_expenses_collection
            .count_documents(doc!("categories.category_id": *category.id.clone()), None)
            .await?;

        if expense_count > 0 {
            return Err(format!(
            "Tried to remove category {:?}, but it still has {} (rendered) expenses attached to it",
            id, expense_count
        )
            .into());
        }

        // everything seems to be in order, proceed with the deletion
        let count = collection
            .delete_one(doc! {"_id":*id}, None)
            .await?
            .deleted_count;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        category: CategoryCreateInput,
    ) -> juniper::FieldResult<Category> {
        require_user(context).await?;
        let collection = context.connection.collection::<Category>("categories");

        let category = category.upgrade(ObjectId::new().into());
        collection.insert_one(&category, None).await?;

        Ok(category)
    }

    pub async fn update(
        context: &Context,
        id: ObjectIdWrapper,
        category: CategoryUpdateInput,
    ) -> juniper::FieldResult<bool> {
        require_user(context).await?;
        let collection = context.connection.collection::<Category>("categories");

        let count = collection
            .update_one(
                doc! {"_id":*id},
                doc! {"$set":  bson::to_bson(&category).unwrap()},
                None,
            )
            .await?
            .modified_count;

        Ok(count == 1)
    }
}
