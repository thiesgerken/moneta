use mongodb::bson::oid::ObjectId;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::{Object, Parameter, ParameterValue};
use rocket_okapi::request::OpenApiFromRequest;
use rocket_okapi::request::RequestHeaderInput;
use std::ops::Deref;

use crate::models::User;

/// Wrapper type for cookie authentication.
/// Use as a rocket Request Guard in order to require an authenticated user.
/// Dereferences to the object id of the user
pub struct CookieUserId(ObjectId);

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

/// declare user as authenticated (without checking anything)
pub fn add_cookie(cookies: &CookieJar<'_>, user: &User) {
    cookies.add_private(Cookie::new("user_id", user.id.to_string()));
}

/// remove authentication cookie from the browser
pub fn remove_cookie(cookies: &CookieJar<'_>) {
    cookies.remove_private(Cookie::named("user_id"));
}
