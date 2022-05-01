use crate::models::Category;
use crate::web::authentication::CookieUserId;


use crate::web::util::log_error_and_500;
use futures::{StreamExt, TryStreamExt};
use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_okapi::openapi;

#[openapi]
#[get("/categories/list?<skip>&<limit>")]
pub async fn list(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<Json<Vec<Category>>, Status> {
    let collection = connection.collection::<Category>("categories");
    let cursor = collection
        .find(None, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .skip(skip.unwrap_or(0))
        .take(limit.unwrap_or(usize::MAX));

    let entries = cursor
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(Json(entries))
}

// #[get("/categories/<id>")]
// pub async fn get(
//     _uid: UserId,
//     connection: DbConn,
//     id: i32,
// ) -> Result<Json<RenderedCategory>, Status> {
//     connection
//         .run(move |c| {
//             let cs = categories::table
//                 .filter(categories::id.eq(id))
//                 .left_join(
//                     category_replacements::table
//                         .on(categories::id.eq(category_replacements::original)),
//                 )
//                 .load::<(Category, Option<CategoryReplacement>)>(c)
//                 .map_err(|e| log_error_and_500(Box::new(e)))?;

//             if let Some((cat, _)) = cs.get(0) {
//                 let cat = cat.clone();
//                 let repl = cs.into_iter().filter_map(|(_, r)| r).collect::<Vec<_>>();

//                 Ok(Json(RenderedCategory::render(cat, &repl)))
//             } else {
//                 Err(Status::NotFound)
//             }
//         })
//         .await
// }

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
