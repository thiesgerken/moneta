use std::io::Write;
use std::ops::Deref;

use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Jsonb,
};
use serde::{Serialize, de::DeserializeOwned};

/// A `jsonb` column holding a serde type.
///
/// diesel maps `jsonb` to `serde_json::Value`, which would push a `from_value` call to every
/// use site. This keeps the model structs holding the shapes they actually mean.
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
pub struct Json<T>(pub T);

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Json(value)
    }
}

impl<T: DeserializeOwned> FromSql<Jsonb, Pg> for Json<T> {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let bytes = value.as_bytes();
        // jsonb is length-prefixed by a version byte; only version 1 exists
        match bytes.first() {
            Some(1) => Ok(Json(serde_json::from_slice(&bytes[1..])?)),
            Some(v) => Err(format!("unsupported jsonb encoding version {v}").into()),
            None => Err("empty jsonb value".into()),
        }
    }
}

impl<T: Serialize + std::fmt::Debug> ToSql<Jsonb, Pg> for Json<T> {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // written by hand rather than delegating to serde_json::Value's impl, which would
        // need the intermediate Value to outlive this call
        out.write_all(&[1])?;
        serde_json::to_writer(out, &self.0)?;
        Ok(IsNull::No)
    }
}
