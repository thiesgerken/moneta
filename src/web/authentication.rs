use mongodb::bson::oid::ObjectId;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use std::ops::Deref;

// TODO: delete whole file

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
