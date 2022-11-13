use std::collections::HashMap;
use std::error::Error;

use crate::graphql::root::require_user;
use crate::graphql::root::Context;
use crate::graphql::util;
use crate::models::rendering::RenderedExpense;
use crate::models::{Category, CategoryCreateInput, CategoryUpdateInput, ObjectIdWrapper};

use dataloader::cached::Loader;
use dataloader::BatchFn;
use futures::TryStreamExt;
use juniper::graphql_object;
use juniper::FieldResult;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};
use mongodb::Database;

#[derive(Debug, Clone)]
pub struct GraphQLCategory {
    data: Category,
}

impl GraphQLCategory {
    pub fn new(data: Category) -> Self {
        Self { data }
    }
}

#[graphql_object(Context = Context, name = "Category")]
impl GraphQLCategory {
    fn id(&self) -> ObjectIdWrapper {
        self.data.id
    }

    fn name(&self) -> &str {
        &self.data.name
    }

    fn description(&self) -> &str {
        &self.data.description
    }

    fn color(&self) -> &Option<String> {
        &self.data.color
    }

    async fn parent(&self, context: &Context) -> FieldResult<Option<GraphQLCategory>> {
        if let Some(parent) = self.data.parent {
            Ok(Some(GraphQLCategory::new(
                context.category_loader.try_load(parent).await?,
            )))
        } else {
            Ok(None)
        }
    }

    fn monthly_limit(&self) -> Option<f64> {
        self.data.monthly_limit
    }
}

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
    ) -> juniper::FieldResult<Vec<GraphQLCategory>> {
        require_user(context).await?;

        let categories = util::list_query::<Category>(
            "categories",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            None,
        )
        .await?
        .1;

        context
            .category_loader
            .prime_many(categories.iter().map(|a| (a.id, a.clone())))
            .await;

        Ok(categories.into_iter().map(GraphQLCategory::new).collect())
    }

    pub async fn get(
        context: &Context,
        id: ObjectIdWrapper,
    ) -> juniper::FieldResult<GraphQLCategory> {
        require_user(context).await?;
        Ok(GraphQLCategory::new(
            context.category_loader.try_load(id).await?,
        ))
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
            .count_documents(doc!("categories.category_id": *category.id), None)
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

pub async fn get_many(
    connection: &Database,
    ids: &[ObjectIdWrapper],
) -> Result<HashMap<ObjectIdWrapper, Category>, Box<dyn Error>> {
    debug!(
        "Performing a coalesced category query for {} ids",
        ids.len()
    );

    let collection = connection.collection::<Category>("categories");
    let categories = collection
        .find(
            doc! {"_id": {"$in": ids.iter().map(|i| **i).collect::<Vec<ObjectId>>()}},
            None,
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(categories
        .into_iter()
        .map(|a| (a.id, a))
        .collect::<HashMap<_, _>>())
}

pub struct CategoryBatcher {
    connection: Database,
}

#[async_trait]
impl BatchFn<ObjectIdWrapper, Category> for CategoryBatcher {
    async fn load(&mut self, keys: &[ObjectIdWrapper]) -> HashMap<ObjectIdWrapper, Category> {
        get_many(&self.connection, keys).await.unwrap_or_default()
    }
}

pub type CategoryLoader = Loader<ObjectIdWrapper, Category, CategoryBatcher>;

pub fn get_category_loader(connection: Database) -> CategoryLoader {
    Loader::new(CategoryBatcher { connection })
}
