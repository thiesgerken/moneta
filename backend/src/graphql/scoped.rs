//! The only way a resolver reaches a table that belongs to somebody.
//!
//! Per-user scoping is the whole authorisation model here: the schema deliberately gives a
//! caller no way to express a scope, so every read has to apply one itself. That was 29
//! hand-written `user_id.eq(user_id)` filters across six resolver modules, and the failure mode
//! of forgetting one is not an error but another person's money on the screen.
//!
//! So the filter is applied here, once per table, and the resolvers ask for a query that is
//! already scoped. `no_resolver_reaches_a_scoped_table_directly` below is what keeps
//! that true: it reads the resolver sources and fails if one of them names a scoped table.
//!
//! Two different guarantees live here and are worth telling apart:
//!
//! - the **rendered views** carry one row per user per upstream entry, so the filter picks the
//!   reader's own rendering of a row they are allowed to see. Without it a shared expense comes
//!   back once per involved user and every total double-counts.
//! - the **owned tables** carry a `user_id` of their own, so the filter is ownership. Without it
//!   a caller reaches rows that are not theirs at all.

use diesel::pg::Pg;
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::{accounts, automation_rules, balances, periodic_rules, seen_deliveries};
use crate::schema_views::{balances_rendered, expense_transactions_rendered, expenses_rendered};

/// The reader's own rendering of the expenses they can see.
pub fn expenses(user_id: Uuid) -> expenses_rendered::BoxedQuery<'static, Pg> {
    expenses_rendered::table
        .into_boxed()
        .filter(expenses_rendered::user_id.eq(user_id))
}

/// The reader's own rendering of the transactions they can see.
///
/// Reading this without the filter double-counts every transaction of an expense that touches
/// two users' accounts, since each user's rendering lists all of them — and filtering on
/// `account_id` instead does not save you, because both copies name the same account.
pub fn expense_transactions(
    user_id: Uuid,
) -> expense_transactions_rendered::BoxedQuery<'static, Pg> {
    expense_transactions_rendered::table
        .into_boxed()
        .filter(expense_transactions_rendered::user_id.eq(user_id))
}

/// The reader's own rendering of the balances they can see.
pub fn balances(user_id: Uuid) -> balances_rendered::BoxedQuery<'static, Pg> {
    balances_rendered::table
        .into_boxed()
        .filter(balances_rendered::user_id.eq(user_id))
}

/// The upstream balance rows this user owns, by way of the account holding them.
///
/// A mirrored account renders another user's balances into this user's books, so owning the
/// rendering is not the same as owning the row behind it — and a write goes to the row. This is
/// the accessor a mutation checks against; [`balances`] is the one a read uses.
pub fn owned_balances(
    user_id: Uuid,
) -> diesel::helper_types::IntoBoxed<
    'static,
    diesel::helper_types::InnerJoin<balances::table, accounts::table>,
    Pg,
> {
    balances::table
        .inner_join(accounts::table)
        .into_boxed()
        .filter(accounts::user_id.eq(user_id))
}

/// The accounts this user owns. Not the same set as the accounts they can *see*: a mirrored
/// account shows up in their rendering without being theirs to write to.
pub fn accounts(user_id: Uuid) -> accounts::BoxedQuery<'static, Pg> {
    accounts::table
        .into_boxed()
        .filter(accounts::user_id.eq(user_id))
}

pub fn automation_rules(user_id: Uuid) -> automation_rules::BoxedQuery<'static, Pg> {
    automation_rules::table
        .into_boxed()
        .filter(automation_rules::user_id.eq(user_id))
}

pub fn periodic_rules(user_id: Uuid) -> periodic_rules::BoxedQuery<'static, Pg> {
    periodic_rules::table
        .into_boxed()
        .filter(periodic_rules::user_id.eq(user_id))
}

pub fn seen_deliveries(user_id: Uuid) -> seen_deliveries::BoxedQuery<'static, Pg> {
    seen_deliveries::table
        .into_boxed()
        .filter(seen_deliveries::user_id.eq(user_id))
}

/// One owned row, addressed by id, in the shape `diesel::update`/`diesel::delete` want.
///
/// A boxed query is not an update target, so a write cannot reuse the readers above — and a
/// write is where forgetting the scope is worst, since it reaches somebody else's row rather
/// than merely reading it. The types are unspeakable by hand, hence the macro.
macro_rules! own {
    ($name:ident, $table:ident) => {
        pub fn $name(
            user_id: Uuid,
            id: Uuid,
        ) -> diesel::helper_types::Filter<
            diesel::helper_types::Find<$table::table, Uuid>,
            diesel::dsl::Eq<$table::user_id, Uuid>,
        > {
            $table::table.find(id).filter($table::user_id.eq(user_id))
        }
    };
}

own!(own_account, accounts);
own!(own_automation_rule, automation_rules);
own!(own_periodic_rule, periodic_rules);

#[cfg(test)]
mod tests {
    /// The tables a resolver has no business naming directly, because reaching one without its
    /// filter is either another user's rows or a double-counted total.
    const SCOPED: [&str; 7] = [
        "expenses_rendered",
        "expense_transactions_rendered",
        "balances_rendered",
        "accounts",
        "automation_rules",
        "periodic_rules",
        "seen_deliveries",
    ];

    /// What an exception has to carry, followed by the reason it is one.
    const MARKER: &str = "scoped-ok:";

    /// The accessors above are only worth anything if they are the way in, and nothing in Rust
    /// says so: `expenses_rendered::table` stays perfectly callable from any resolver. So this
    /// reads the resolver sources and fails on one that names a scoped table.
    ///
    /// A grep as a test is unusual, and it is the cheap half of a bargain: the alternative that
    /// the compiler could enforce is a newtype wrapper around every table, which is a great deal
    /// of code for the same sentence. What matters is that forgetting the filter stops being
    /// something you can do quietly.
    ///
    /// `insert_into` is exempt without a marker: a write that names its own `user_id` cannot
    /// reach a row that is somebody else's. Everything else needs `// scoped-ok: <why>`.
    #[test]
    fn no_resolver_reaches_a_scoped_table_directly() {
        let mut offences = Vec::new();

        for entry in std::fs::read_dir("src/graphql").expect("the resolver directory is there") {
            let path = entry.expect("readable entry").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            // this file is where the filters live, so it is the one place they are absent
            if path.file_name().is_some_and(|n| n == "scoped.rs") {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("readable source");

            // a marker sits either at the end of the offending line or in the comment above it,
            // since most of these need a sentence rather than a clause
            let mut marked = false;

            for (number, line) in source.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with("//") {
                    marked |= line.contains(MARKER);
                    continue;
                }

                let exempt = marked || line.contains(MARKER) || line.contains("insert_into");
                // the marker covers the statement it introduces and nothing after it
                marked = false;
                if exempt {
                    continue;
                }

                for table in SCOPED {
                    if line.contains(&format!("{table}::table")) {
                        offences.push(format!(
                            "{}:{}: {} -- use graphql::scoped, or say why not with `// {MARKER} ...`",
                            path.display(),
                            number + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }

        assert!(
            offences.is_empty(),
            "a resolver reaches a scoped table directly:\n{}",
            offences.join("\n")
        );
    }

    /// A marker with no reason after it is a way to switch the test off, which is not what it is
    /// for. Every exception has to say what makes it one.
    #[test]
    fn every_exception_gives_a_reason() {
        for entry in std::fs::read_dir("src/graphql").expect("the resolver directory is there") {
            let path = entry.expect("readable entry").path();
            // this file spells the marker out to look for it, which is not an exemption
            if path.file_name().is_some_and(|n| n == "scoped.rs") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };

            for (number, line) in source.lines().enumerate() {
                let Some((_, reason)) = line.split_once(MARKER) else {
                    continue;
                };

                assert!(
                    reason.trim().len() > 15,
                    "{}:{} exempts itself without saying why",
                    path.display(),
                    number + 1
                );
            }
        }
    }
}
