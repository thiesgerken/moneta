use crate::models::*;
use crate::rendering::RenderedCategory;
use crate::schema::*;
use crate::web::user::UserId;
use crate::web::DbConn;

use crate::web::util::log_error_and_500;
use diesel::prelude::*;
use rocket::http::Status;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

#[get("/categories")]
pub async fn list(_uid: UserId, connection: DbConn) -> Result<Json<Vec<RenderedCategory>>, Status> {
    connection
        .run(move |c| {
            let cs = categories::table
                .load::<Category>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;
            let repl = category_replacements::table
                .load::<CategoryReplacement>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            Ok(Json(
                cs.into_iter()
                    .map(|c| RenderedCategory::render(c, &repl))
                    .collect(),
            ))
        })
        .await
}

#[get("/categories/<id>")]
pub async fn get(
    _uid: UserId,
    connection: DbConn,
    id: i32,
) -> Result<Json<RenderedCategory>, Status> {
    connection
        .run(move |c| {
            let cs = categories::table
                .filter(categories::id.eq(id))
                .left_join(
                    category_replacements::table
                        .on(categories::id.eq(category_replacements::original)),
                )
                .load::<(Category, Option<CategoryReplacement>)>(c)
                .map_err(|e| log_error_and_500(Box::new(e)))?;

            if let Some((cat, _)) = cs.get(0) {
                let cat = cat.clone();
                let repl = cs.into_iter().filter_map(|(_, r)| r).collect::<Vec<_>>();

                Ok(Json(RenderedCategory::render(cat, &repl)))
            } else {
                Err(Status::NotFound)
            }
        })
        .await
}

// #[delete("/balances/<id>")]
// pub fn delete(uid: UserId, connection: DbConn, id: i32) -> Result<(), Status> {
//     todo!() // TODO
// }

// #[put("/balances/<id>", data = "<balances>")]
// pub fn update(
//     uid: UserId,
//     connection: DbConn,
//     id: i32,
//     account: Json<RenderedBalance>,
// ) -> Result<(), Status> {
//     todo!() // TODO
// }

// #[post("/balances", data = "<balances>")]
// pub fn create(
//     uid: UserId,
//     connection: DbConn,
//     account: Json<NewBalance>,
// ) -> Result<Json<RenderedBalance>, Status> {
//     todo!() // TODO
// }
