use crate::queries;
use crate::rendering::RenderedAccount;
use crate::web::user::UserId;
use crate::web::util::log_error_and_500;
use crate::web::DbConn;

use rocket::http::Status;
use rocket_contrib::json::Json;

#[get("/accounts?<offset>&<count>")]
pub async fn list(
    _uid: UserId,
    connection: DbConn,
    offset: Option<i32>,
    count: Option<i32>,
) -> Result<Json<Vec<RenderedAccount>>, Status> {
    connection
        .run(move |c| {
            let accs = queries::accounts(c, offset, count)
                .map_err(|e| log_error_and_500(Box::new(e)))?
                .into_iter()
                .map(|(account, syncing)| RenderedAccount {
                    info: account,
                    synchronization: syncing,
                })
                .collect();

            Ok(Json(accs))
        })
        .await
}

#[get("/accounts/<id>")]
pub async fn get(
    _uid: UserId,
    connection: DbConn,
    id: i32,
) -> Result<Json<RenderedAccount>, Status> {
    connection
        .run(move |c| {
            let (account, syncing) = queries::account_by_id(c, id)
                .map_err(|e| log_error_and_500(Box::new(e)))?
                .ok_or(Status::NotFound)?;

            Ok(Json(RenderedAccount {
                info: account,
                synchronization: syncing,
            }))
        })
        .await
}

// #[delete("/accounts/<id>")]
// pub fn delete(uid: UserId, connection: DbConn, id: i32) -> Result<(), Status> {
//     // let row_count = diesel::delete(
//     //     accounts::table
//     //         .filter(accounts::user_id.eq(*uid))
//     //         .filter(accounts::id.eq(id)),
//     // )
//     // .execute(&*connection)
//     // .map_err(|e| log_error_and_500(Box::new(e)))?;

//     // if row_count == 0 {
//     //     Err(Status::NotFound)
//     // } else {
//     //     info!("Deleted record {} from the account table", id);
//     //     Ok(())
//     // }

//     todo!() // TODO
// }

// #[put("/accounts/<id>", data = "<account>")]
// pub fn update(
//     uid: UserId,
//     connection: DbConn,
//     id: i32,
//     account: Json<Account>,
// ) -> Result<(), Status> {
//     // let mut acc = account.0;
//     // acc.user_id = *uid;

//     // let row_count = diesel::update(
//     //     accounts::table
//     //         .filter(accounts::id.eq(id))
//     //         .filter(accounts::user_id.eq(*uid)),
//     // )
//     // .set(acc)
//     // .execute(&*connection)
//     // .map_err(|e| log_error_and_500(Box::new(e)))?;

//     // if row_count == 0 {
//     //     Err(Status::NotFound)
//     // } else {
//     //     info!("updated record {} from the account table", id);
//     //     Ok(())
//     // }

//     todo!() // TODO
// }

// #[post("/accounts", data = "<account>")]
// pub fn create(
//     uid: UserId,
//     connection: DbConn,
//     account: Json<NewAccount>,
// ) -> Result<Json<Account>, Status> {
//     // let mut acc = account.0;
//     // acc.user_id = *uid;

//     // let t: Account = diesel::insert_into(crate::schema::accounts::table)
//     //     .values(&acc)
//     //     .get_result(&*connection)
//     //     .map_err(|e| log_error_and_500(Box::new(e)))?;

//     // Ok(Json(t))

//     todo!() // TODO
// }
