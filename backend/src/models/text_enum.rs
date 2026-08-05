//! Maps the small closed enums onto `text` columns.
//!
//! The schema stores them as text with a CHECK rather than as postgres enum types, so adding
//! a variant stays a one-line migration. The database rejects anything outside the CHECK, and
//! `from_db` rejects anything the CHECK was widened to but Rust does not know about yet.

/// Generates the text mapping and the diesel glue for a fieldless enum.
///
/// The literals are the on-disk representation and have to stay in step with the CHECK
/// constraint in the migration; a mismatch surfaces the first time a row is written.
macro_rules! text_enum {
    ($ty:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $ty {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub fn from_db(s: &str) -> Result<Self, String> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", stringify!($ty), ": {}"), other)),
                }
            }
        }

        impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::pg::Pg> for $ty {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                use std::io::Write;
                out.write_all(self.as_str().as_bytes())?;
                Ok(diesel::serialize::IsNull::No)
            }
        }

        impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg> for $ty {
            fn from_sql(
                value: diesel::pg::PgValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                Ok(Self::from_db(std::str::from_utf8(value.as_bytes())?)?)
            }
        }
    };
}

pub(crate) use text_enum;
