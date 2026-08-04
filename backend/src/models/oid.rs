use std::ops::Deref;

use juniper::GraphQLScalar;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(GraphQLScalar)]
#[graphql(
    to_output_with = to_output,
    from_input_with = from_input,
    parse_token(String)
)]
#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ObjectIdWrapper(pub ObjectId);

fn to_output(v: &ObjectIdWrapper) -> String {
    v.0.to_hex()
}

fn from_input(s: &str) -> Result<ObjectIdWrapper, Box<str>> {
    ObjectId::parse_str(s)
        .map(ObjectIdWrapper)
        .map_err(|e| format!("Failed to parse `ObjectIdWrapper`: {e}").into())
}

impl Deref for ObjectIdWrapper {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ObjectIdWrapper> for ObjectId {
    fn from(o: ObjectIdWrapper) -> Self {
        o.0
    }
}

impl From<ObjectId> for ObjectIdWrapper {
    fn from(o: ObjectId) -> Self {
        ObjectIdWrapper(o)
    }
}
