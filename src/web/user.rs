use crate::models::{ObjectIdWrapper, User};
use crate::web::authentication;
use crate::{queries, verify_password};

use futures::{StreamExt, TryStreamExt};
use log::{info, warn};
use mongodb::bson::doc;
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};
use serde::{Deserialize, Serialize};

use super::authentication::CookieUserId;
use super::util::log_error_and_500;

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    #[serde(rename = "_id")]
    id: ObjectIdWrapper,
    name: String,
    full_name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    user: String,
    password: String,
}

impl UserInfo {
    pub fn new(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            full_name: user.full_name,
        }
    }
}

#[openapi]
#[get("/user")]
pub async fn info(
    uid: CookieUserId,
    connection: &State<mongodb::Database>,
) -> Result<Json<UserInfo>, Status> {
    let user = queries::query_element_by_id(connection, "users", *uid)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::InternalServerError)?;

    Ok(Json(UserInfo::new(user)))
}

#[openapi]
#[get("/user/logout")]
pub fn logout(cookies: &CookieJar) -> Status {
    authentication::remove_cookie(cookies);
    Status::Ok
}

#[openapi]
#[post("/user/login", data = "<login>")]
pub async fn login(
    connection: &State<mongodb::Database>,
    cookies: &CookieJar<'_>,
    login: Json<LoginData>,
) -> Result<Json<UserInfo>, Status> {
    if let Some(user) = queries::query_user_by_name(connection, &login.user)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
    {
        if verify_password(&login.password, &user.password_hash) {
            info!("Successful login with username '{}'", &login.user);
            authentication::add_cookie(cookies, &user);
            Ok(Json(UserInfo::new(user)))
        } else {
            warn!(
                "Attempt to login with wrong password for user '{}'",
                &login.user
            );

            Err(Status::Unauthorized)
        }
    } else {
        warn!(
            "Attempt to login with non-existing username '{}'",
            &login.user
        );

        Err(Status::Unauthorized)
    }
}

#[openapi]
#[get("/user/list?<skip>&<limit>")]
pub async fn list(
    _uid: CookieUserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<Json<Vec<UserInfo>>, Status> {
    let collection = connection.collection::<User>("users");
    let cursor = collection
        .find(None, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .skip(skip.unwrap_or(0))
        .take(limit.unwrap_or(usize::MAX));

    let users = cursor
        .and_then(|u| async { Ok(UserInfo::new(u)) })
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;

    Ok(Json(users))
}
