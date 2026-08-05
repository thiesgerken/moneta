//! Hand-written table definitions for the rendered views.
//!
//! `diesel print-schema` skips them because a view has no primary key to infer, and it
//! rewrites `schema.rs` wholesale, so they live here instead of being pasted back in after
//! every regeneration. The keys named below are the ones the views are unique on, not
//! anything postgres enforces.

diesel::table! {
    use diesel::sql_types::*;

    expenses_rendered (id, user_id) {
        id -> Uuid,
        upstream_id -> Uuid,
        user_id -> Uuid,
        title -> Text,
        store -> Text,
        description -> Text,
        comments -> Text,
        reference_amount -> Nullable<Int4>,
        booking_start -> Timestamptz,
        booking_end -> Timestamptz,
        is_template -> Bool,
        is_tax_relevant -> Bool,
        is_unchecked -> Bool,
        is_incomplete -> Bool,
        is_extraordinary -> Bool,
        receipts -> Jsonb,
        positions -> Jsonb,
        categories -> Jsonb,
        transactions -> Jsonb,
        // deliberately not nullable: the view leaves this null when a fractional transaction
        // has no reference amount to resolve against, and reading it must fail rather than
        // report a total that quietly omits one
        effective_amount -> Float8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    expense_transactions_rendered (transaction_id, user_id) {
        transaction_id -> Uuid,
        expense_id -> Uuid,
        user_id -> Uuid,
        idx -> Int4,
        date -> Timestamptz,
        comments -> Text,
        statement -> Text,
        account_id -> Uuid,
        amount -> Nullable<Int4>,
        fraction -> Nullable<Float8>,
        // see expenses_rendered.effective_amount
        calculated_amount -> Float8,
        effective_amount -> Float8,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    balances_rendered (id, user_id) {
        id -> Uuid,
        upstream_id -> Uuid,
        user_id -> Uuid,
        date -> Timestamptz,
        account_id -> Uuid,
        amount -> Int4,
        comments -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    expense_users (expense_id, user_id) {
        expense_id -> Uuid,
        user_id -> Uuid,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    balances_rendered,
    expense_transactions_rendered,
    expense_users,
    expenses_rendered,
);

// the macro only takes bare identifiers, so the upstream tables have to be in scope by name
use crate::schema::{accounts, categories, expenses};

diesel::allow_tables_to_appear_in_same_query!(expenses_rendered, accounts);
diesel::allow_tables_to_appear_in_same_query!(expenses_rendered, categories);
diesel::allow_tables_to_appear_in_same_query!(expenses_rendered, expenses);
diesel::allow_tables_to_appear_in_same_query!(balances_rendered, accounts);
diesel::allow_tables_to_appear_in_same_query!(expense_transactions_rendered, accounts);
