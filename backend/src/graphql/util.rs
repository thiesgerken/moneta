use juniper::GraphQLEnum;
use serde_json::Value;

/// Which way a list is ordered.
///
/// Lists used to take a mongo sort document built in the browser (`{"accrualStart": -1}`) and
/// a mongo filter document alongside it. Both are gone: the client now names a column from a
/// per-entity enum and a direction, so nothing it sends can reach a column the resolver did
/// not offer, and there is no query language to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

/// Applies `.order_by()` in the requested direction.
///
/// diesel's ascending and descending orderings are different types, so a plain `if` cannot
/// return both arms; each has to be applied to an already-boxed query.
#[macro_export]
macro_rules! order_by {
    ($query:expr, $direction:expr, $column:expr) => {
        match $direction {
            $crate::graphql::util::SortDirection::Asc => $query.order_by($column.asc()),
            $crate::graphql::util::SortDirection::Desc => $query.order_by($column.desc()),
        }
    };
}

/// Clamps the window a list query may ask for.
///
/// Without a ceiling a client can pull the whole expense table in one request; without a
/// floor a negative limit reaches diesel as a nonsensical LIMIT.
pub fn page(limit: Option<i32>, skip: Option<i32>) -> (i64, i64) {
    const MAX_LIMIT: i64 = 1000;

    let limit = limit
        .map(i64::from)
        .unwrap_or(MAX_LIMIT)
        .clamp(0, MAX_LIMIT);
    let offset = skip.map(i64::from).unwrap_or(0).max(0);

    (limit, offset)
}

/// Wraps a free-text term for a case-insensitive `ILIKE`, escaping the wildcards so that
/// searching for a literal `%` does not match every row.
pub fn ilike_pattern(term: &str) -> String {
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

/// One place where two serialised states differ.
pub struct JsonChange {
    /// Slash-joined, so a reader can name both the list element and the field inside it:
    /// `transactions/0/amount`.
    pub path: String,
    /// `None` where the field is not on that side at all, which is how an added or removed
    /// list element arrives -- one change carrying the whole element rather than one per
    /// field inside it. A field that is present and JSON `null` is `Some(Value::Null)`.
    pub old: Option<Value>,
    pub new: Option<Value>,
}

/// Every leaf that differs between two states.
///
/// A text diff of the two renderings is exact and unreadable: a changed amount arrives as a
/// dozen lines of context around one number, and nothing downstream can tell that the number
/// is cents and belongs on a transaction. Walking the two values instead gives the reader a
/// field it can name and a value it can format.
pub fn json_changes(left: &Value, right: &Value) -> Vec<JsonChange> {
    let mut out = Vec::new();
    walk(&mut Vec::new(), Some(left), Some(right), &mut out);
    out
}

fn walk(
    path: &mut Vec<String>,
    left: Option<&Value>,
    right: Option<&Value>,
    out: &mut Vec<JsonChange>,
) {
    match (left, right) {
        (Some(Value::Object(l)), Some(Value::Object(r))) => {
            // left's order first, since that is the order the struct declares its fields in
            // and the reader shows them in the order they arrive
            let keys = l.keys().chain(r.keys().filter(|k| !l.contains_key(*k)));
            for key in keys.cloned().collect::<Vec<_>>() {
                path.push(key.clone());
                walk(path, l.get(&key), r.get(&key), out);
                path.pop();
            }
        }
        // ponytail: paired up by position, so inserting a transaction ahead of another reports
        // both as changed rather than one as added. Lists here hold a handful of elements and
        // things are appended to them; matching by identity is only worth it if that stops
        // being true.
        (Some(Value::Array(l)), Some(Value::Array(r))) => {
            for i in 0..l.len().max(r.len()) {
                path.push(i.to_string());
                walk(path, l.get(i), r.get(i), out);
                path.pop();
            }
        }
        _ if left == right => {}
        _ => out.push(JsonChange {
            path: path.join("/"),
            old: left.cloned(),
            new: right.cloned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_term_cannot_smuggle_wildcards() {
        // otherwise searching for "50%" matches every row
        assert_eq!(ilike_pattern("50%"), r"%50\%%");
        assert_eq!(ilike_pattern("a_b"), r"%a\_b%");
        assert_eq!(ilike_pattern(r"back\slash"), r"%back\\slash%");
    }

    #[test]
    fn paging_is_bounded() {
        assert_eq!(page(None, None), (1000, 0));
        assert_eq!(page(Some(50), Some(10)), (50, 10));
        // a client cannot ask for the whole table, nor for a negative window
        assert_eq!(page(Some(i32::MAX), None), (1000, 0));
        assert_eq!(page(Some(-5), Some(-5)), (0, 0));
    }

    fn changed(left: Value, right: Value) -> Vec<(String, String, String)> {
        json_changes(&left, &right)
            .into_iter()
            .map(|c| {
                let side = |v: Option<Value>| v.map(|v| v.to_string()).unwrap_or("-".into());
                (c.path, side(c.old), side(c.new))
            })
            .collect()
    }

    #[test]
    fn only_the_leaf_that_moved_is_reported() {
        assert_eq!(
            changed(
                serde_json::json!({"title": "Rewe", "amount": -1000}),
                serde_json::json!({"title": "Rewe", "amount": -1200}),
            ),
            vec![("amount".into(), "-1000".into(), "-1200".into())]
        );
    }

    #[test]
    fn a_nested_field_carries_its_path() {
        assert_eq!(
            changed(
                serde_json::json!({"transactions": [{"amount": -1000, "fraction": null}]}),
                serde_json::json!({"transactions": [{"amount": null, "fraction": 1.0}]}),
            ),
            vec![
                (
                    "transactions/0/amount".into(),
                    "-1000".into(),
                    "null".into()
                ),
                (
                    "transactions/0/fraction".into(),
                    "null".into(),
                    "1.0".into()
                ),
            ]
        );
    }

    /// The case the whole thing exists for: splitting an expense appends a transaction, and
    /// that has to read as one addition rather than as seven fields appearing out of nowhere.
    #[test]
    fn an_added_element_arrives_whole() {
        assert_eq!(
            changed(
                serde_json::json!({"transactions": [{"amount": -1000}]}),
                serde_json::json!({"transactions": [{"amount": -1000}, {"fraction": -0.5}]}),
            ),
            vec![(
                "transactions/1".into(),
                "-".into(),
                "{\"fraction\":-0.5}".into()
            )]
        );
    }

    #[test]
    fn an_unchanged_state_has_nothing_to_report() {
        let state = serde_json::json!({"title": "Rewe", "transactions": [{"amount": -1000}]});
        assert!(json_changes(&state, &state).is_empty());
    }
}
