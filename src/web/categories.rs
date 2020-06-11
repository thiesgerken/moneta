use crate::models::*;
use crate::queries;
use crate::rendering::RenderedCategory;
use crate::schema::*;
use crate::web::user::UserId;
use crate::web::DbConn;

use diesel::prelude::*;
use log::info;
use rocket::http::Status;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

#[get("/categories")]
pub fn list(_uid: UserId, connection: DbConn) -> Option<Json<Vec<RenderedCategory>>> {
    let cs = categories::table.load::<Category>(&*connection).ok()?;
    let repl = category_replacements::table
        .load::<CategoryReplacement>(&*connection)
        .ok()?;

    Some(Json(
        cs.into_iter()
            .map(|c| RenderedCategory::render(c, &repl))
            .collect(),
    ))
}

#[get("/categories/<id>")]
pub fn get(_uid: UserId, connection: DbConn, id: i32) -> Option<Json<RenderedCategory>> {
    let cs = categories::table
        .filter(categories::id.eq(id))
        .left_join(
            category_replacements::table.on(categories::id.eq(category_replacements::original)),
        )
        .load::<(Category, Option<CategoryReplacement>)>(&*connection)
        .ok()?;

    if let Some((cat, _)) = cs.get(0) {
        let cat = cat.clone();
        let repl = cs.into_iter().filter_map(|(_, r)| r).collect::<Vec<_>>();

        Some(Json(RenderedCategory::render(cat, &repl)))
    } else {
        None
    }
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
