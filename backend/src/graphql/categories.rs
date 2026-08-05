use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::graphql::root::{Context, require_user};
use crate::graphql::util::{self, SortDirection};
use crate::models::{Category, CategoryCreateInput, CategoryUpdateInput};
use crate::order_by;
use crate::schema::{categories, expense_categorizations};

use dataloader::BatchFn;
use dataloader::cached::Loader;
use juniper::{FieldResult, GraphQLEnum, graphql_object};

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
    fn id(&self) -> Uuid {
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
        if let Some(parent) = self.data.parent_id {
            Ok(Some(GraphQLCategory::new(
                context.category_loader.try_load(parent).await?,
            )))
        } else {
            Ok(None)
        }
    }

    /// desired maximum spending per month, in cents
    fn monthly_limit(&self) -> Option<i32> {
        self.data.monthly_limit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum CategorySortField {
    #[default]
    Name,
    MonthlyLimit,
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
        search: Option<String>,
        sort_by: Option<CategorySortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<Vec<GraphQLCategory>> {
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();

        let mut query = categories::table.into_boxed();
        if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
            let pattern = util::ilike_pattern(term.trim());
            query = query.filter(
                categories::name
                    .ilike(pattern.clone())
                    .or(categories::description.ilike(pattern)),
            );
        }

        let query = match sort_by.unwrap_or_default() {
            CategorySortField::Name => order_by!(query, direction, categories::name),
            CategorySortField::MonthlyLimit => {
                order_by!(query, direction, categories::monthly_limit)
            }
        };

        let found = query
            .limit(limit)
            .offset(offset)
            .select(Category::as_select())
            .load(conn)
            .await?;

        context
            .category_loader
            .prime_many(found.iter().map(|a| (a.id, a.clone())))
            .await;

        Ok(found.into_iter().map(GraphQLCategory::new).collect())
    }

    pub async fn get(context: &Context, id: Uuid) -> FieldResult<GraphQLCategory> {
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
    pub async fn delete(context: &Context, id: Uuid) -> FieldResult<bool> {
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        // The foreign key is ON DELETE RESTRICT, so the database refuses this regardless. The
        // check stays so the user is told how many expenses are in the way rather than a
        // constraint name they cannot act on.
        let attached: i64 = expense_categorizations::table
            .filter(expense_categorizations::category_id.eq(id))
            .count()
            .get_result(conn)
            .await?;

        if attached > 0 {
            return Err(format!(
                "Tried to remove category {id}, but it still has {attached} expenses attached to it"
            )
            .into());
        }

        let count = diesel::delete(categories::table.find(id))
            .execute(conn)
            .await?;
        Ok(count == 1)
    }

    pub async fn create(context: &Context, category: CategoryCreateInput) -> FieldResult<Category> {
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        let category = category.upgrade(Uuid::now_v7());
        diesel::insert_into(categories::table)
            .values(&category)
            .execute(conn)
            .await?;

        Ok(category)
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        category: CategoryUpdateInput,
    ) -> FieldResult<bool> {
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        let count = diesel::update(categories::table.find(id))
            .set((
                categories::name.eq(category.name),
                categories::description.eq(category.description),
                categories::color.eq(category.color),
                categories::parent_id.eq(category.parent_id),
                categories::monthly_limit.eq(category.monthly_limit),
            ))
            .execute(conn)
            .await?;

        Ok(count == 1)
    }
}

pub struct CategoryBatcher {
    pool: crate::Pool,
}

impl BatchFn<Uuid, Category> for CategoryBatcher {
    async fn load(&mut self, keys: &[Uuid]) -> HashMap<Uuid, Category> {
        let Ok(mut conn) = self.pool.get().await else {
            return HashMap::new();
        };

        categories::table
            .filter(categories::id.eq_any(keys))
            .select(Category::as_select())
            .load(&mut conn)
            .await
            .map(|found| found.into_iter().map(|c| (c.id, c)).collect())
            .unwrap_or_default()
    }
}

pub type CategoryLoader = Loader<Uuid, Category, CategoryBatcher>;

pub fn get_category_loader(pool: crate::Pool) -> CategoryLoader {
    Loader::new(CategoryBatcher { pool })
}
