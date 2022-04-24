use crate::models::{ObjectIdWrapper, User};
use crate::{queries, verify_password};

use futures::{StreamExt, TryStreamExt};
use log::{info, warn};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use rocket::{Request, State};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::{Object, Parameter, ParameterValue};
use rocket_okapi::request::OpenApiFromRequest;
use rocket_okapi::request::RequestHeaderInput;
use rocket_okapi::{openapi, JsonSchema};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

use super::util::log_error_and_500;

impl Deref for CookieUserId {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> OpenApiFromRequest<'a> for CookieUserId {
    fn from_request_input(
        gen: &mut OpenApiGenerator,
        _name: String,
        required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let schema = gen.json_schema::<String>();
        Ok(RequestHeaderInput::Parameter(Parameter {
            name: "user_id".to_owned(),
            location: "cookie".to_owned(),
            description: None,
            required,
            deprecated: false,
            allow_empty_value: true,
            value: ParameterValue::Schema {
                style: None,
                explode: None,
                allow_reserved: false,
                schema,
                example: None,
                examples: None,
            },
            extensions: Object::default(),
        }))
    }
}

pub struct CookieUserId(pub ObjectId);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CookieUserId {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let id = request
            .cookies()
            .get_private("user_id")
            .and_then(|c| c.value().parse().ok());

        match id {
            Some(id) => Outcome::Success(CookieUserId(id)),
            None => Outcome::Failure((Status::Unauthorized, ())),
        }
    }
}

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
    cookies.remove_private(Cookie::named("user_id"));
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
