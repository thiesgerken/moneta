use juniper::GraphQLEnum;
use similar::TextDiff;

/// Which way a list is ordered.
///
/// Lists used to take a mongo sort document built in the browser (`{"bookingStart": -1}`) and
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

pub fn json_diff<T: serde::Serialize>(
    left: &T,
    right: &T,
) -> Result<String, Box<dyn std::error::Error>> {
    let left_json = serde_json::to_string_pretty(left)? + "\n";
    let right_json = serde_json::to_string_pretty(right)? + "\n";

    let diff = TextDiff::from_lines(&left_json, &right_json);
    Ok(diff.unified_diff().context_radius(3).to_string())
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

    #[test]
    fn json_diff_reports_only_what_changed() {
        let a = serde_json::json!({"title": "Rewe", "amount": -1000});
        let b = serde_json::json!({"title": "Rewe", "amount": -1200});
        let d = json_diff(&a, &b).unwrap();

        assert!(d.contains("-1000"), "{d}");
        assert!(d.contains("-1200"), "{d}");
    }
}
