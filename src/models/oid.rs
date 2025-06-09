use std::ops::Deref;

use juniper::{
    GraphQLScalar, InputValue, ScalarValue, Value,

};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(GraphQLScalar)]
#[graphql(
    to_output_with = to_output,
    from_input_with = from_input,
    parse_token( String)
)]
#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ObjectIdWrapper(pub ObjectId);


fn to_output<S: ScalarValue>(v: &ObjectIdWrapper) -> Value<S> {
    Value::scalar(v.0.to_hex())
}

fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<ObjectIdWrapper, String> {
    match *v {
        InputValue::Scalar(ref s) => s
            .as_string()
            .and_then(|s| ObjectId::parse_str(s).ok())
            .map(ObjectIdWrapper)
            .ok_or_else(|| format!("Expected a string or integer, got {:?}", v)),
        _ => Err(format!("Expected a string or integer, got {:?}", v)),
    }
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
