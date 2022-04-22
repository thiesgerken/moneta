use crate::models::User;
use crate::verify_password;

use log::{info, warn};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use rocket::{Request, State};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::ops::Deref;

use super::util::log_error_and_500;

impl Deref for UserId {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct UserId(pub ObjectId);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserId {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let id = request
            .cookies()
            .get_private("user_id")
            .and_then(|c| c.value().parse().ok());

        match id {
            Some(id) => Outcome::Success(UserId(id)),
            None => Outcome::Failure((Status::Unauthorized, ())),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    full_name: String,
}

#[derive(Deserialize)]
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

pub async fn query_user_by_name(
    connection: &mongodb::Database,
    user_name: &str,
) -> Result<Option<User>, mongodb::error::Error> {
    let collection = connection.collection::<User>("users");
    let filter = doc! {"name":user_name};
    collection.find_one(filter, None).await
}

#[get("/user")]
pub async fn info(
    uid: UserId,
    connection: &State<mongodb::Database>,
) -> Result<Json<UserInfo>, Status> {
    let user = query_element_by_id(connection, "users", *uid)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
        .ok_or(Status::InternalServerError)?;

    Ok(Json(UserInfo::new(user)))
}

#[get("/user/logout")]
pub fn logout(cookies: &CookieJar) -> Status {
    cookies.remove_private(Cookie::named("user_id"));
    Status::Ok
}

#[post("/user/login", data = "<login>")]
pub async fn login(
    connection: &State<mongodb::Database>,
    cookies: &CookieJar<'_>,
    login: Json<LoginData>,
) -> Result<Json<UserInfo>, Status> {
    if let Some(user) = query_user_by_name(connection, &login.user)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?
    {
        if verify_password(&login.password, &user.password_hash) {
            info!("Successful login with username '{}'", &login.user);
            cookies.add_private(Cookie::new("user_id", user.id.to_string()));

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

use futures::stream::StreamExt;

// TODO: implement & use pagination on the cursor through limit and skip
#[get("/user/list?<skip>&<limit>")]
pub async fn list(
    _uid: UserId,
    connection: &State<mongodb::Database>,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<Json<Vec<UserInfo>>, Status> {
    let collection = connection.collection::<User>("users");
    let mut cursor = collection
        .find(None, None)
        .await
        .map_err(|e| log_error_and_500(Box::new(e)))?;
    if let Some(skip) = skip {
        cursor = cursor.skip(skip);
    }

    if let Some(limit) = limit {
        cursor = cursor.take(limit);
    }

    Err(Status::Accepted)
}
