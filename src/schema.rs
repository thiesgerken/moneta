#![allow(warnings)]

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    account_synchronizations (account1) {
        account1 -> Int4,
        account2 -> Int4,
        user1 -> Int4,
        user2 -> Int4,
        invert -> Bool,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    accounts (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Text,
        description -> Text,
        color -> Nullable<Text>,
        iban -> Nullable<Text>,
        kind -> Account_kind,
        availability -> Account_availability,
        risk -> Account_risk,
        hidden -> Bool,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    balances (id) {
        id -> Int4,
        account_id -> Int4,
        date -> Timestamptz,
        amount -> Int8,
        comment -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    categories (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Text,
        description -> Text,
        color -> Nullable<Text>,
        parent -> Nullable<Int4>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    category_replacements (user_id, original) {
        user_id -> Int4,
        original -> Int4,
        replacement -> Int4,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    delivery_rules (id) {
        id -> Int4,
        user_id -> Int4,
        priority -> Int4,
        template_id -> Int4,
        account_id -> Nullable<Int4>,
        amount -> Nullable<Int8>,
        statement_regex -> Text,
        last_match -> Nullable<Timestamptz>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    expense_categories (expense_id, category_id) {
        expense_id -> Int4,
        category_id -> Int4,
        weight -> Float8,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    expense_events (id) {
        id -> Int4,
        expense_id -> Int4,
        user_id -> Nullable<Int4>,
        date -> Timestamptz,
        tool -> Text,
        automatic -> Bool,
        event_type -> Expense_event_type,
        event_target -> Expense_event_target,
        payload -> Nullable<Text>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    expense_receipts (id) {
        id -> Int4,
        expense_id -> Int4,
        file_name -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    expense_transactions (id) {
        id -> Int4,
        expense_id -> Int4,
        account_id -> Int4,
        date -> Timestamptz,
        amount -> Nullable<Int8>,
        fraction -> Nullable<Float8>,
        comments -> Text,
        statement -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    expenses (id) {
        id -> Int4,
        title -> Text,
        description -> Text,
        store -> Text,
        comments -> Text,
        booking_start -> Timestamptz,
        booking_end -> Timestamptz,
        is_deleted -> Bool,
        is_template -> Bool,
        is_preliminary -> Bool,
        is_tax_relevant -> Bool,
        is_unchecked -> Bool,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    users (id) {
        id -> Int4,
        name -> Text,
        full_name -> Text,
        hash -> Text,
    }
}

joinable!(accounts -> users (user_id));
joinable!(balances -> accounts (account_id));
joinable!(categories -> users (user_id));
joinable!(category_replacements -> users (user_id));
joinable!(delivery_rules -> accounts (account_id));
joinable!(delivery_rules -> expenses (template_id));
joinable!(delivery_rules -> users (user_id));
joinable!(expense_categories -> categories (category_id));
joinable!(expense_categories -> expenses (expense_id));
joinable!(expense_events -> expenses (expense_id));
joinable!(expense_events -> users (user_id));
joinable!(expense_receipts -> expenses (expense_id));
joinable!(expense_transactions -> accounts (account_id));
joinable!(expense_transactions -> expenses (expense_id));

allow_tables_to_appear_in_same_query!(
    account_synchronizations,
    accounts,
    balances,
    categories,
    category_replacements,
    delivery_rules,
    expense_categories,
    expense_events,
    expense_receipts,
    expense_transactions,
    expenses,
    users,
);
