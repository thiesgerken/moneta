// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;

    accounts (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Text,
        description -> Text,
        color -> Nullable<Text>,
        iban -> Nullable<Text>,
        kind -> Text,
        availability -> Text,
        risk -> Text,
        opened -> Timestamptz,
        closed -> Nullable<Timestamptz>,
        ownership_fraction -> Float8,
        sync_account_id -> Nullable<Uuid>,
        sync_invert -> Nullable<Bool>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    automation_rules (id) {
        id -> Uuid,
        user_id -> Uuid,
        title -> Text,
        template_id -> Uuid,
        priority -> Int4,
        regex_time -> Nullable<Text>,
        regex_statement -> Nullable<Text>,
        filter_amount -> Nullable<Int4>,
        filter_account_id -> Nullable<Uuid>,
        last_delivery -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    balances (id) {
        id -> Uuid,
        account_id -> Uuid,
        date -> Timestamptz,
        amount -> Int4,
        comments -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    categories (id) {
        id -> Uuid,
        name -> Text,
        description -> Text,
        color -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        monthly_limit -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    expense_categorizations (expense_id, category_id) {
        expense_id -> Uuid,
        category_id -> Uuid,
        weight -> Float8,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    expense_changes (id) {
        id -> Uuid,
        target_id -> Uuid,
        user_id -> Nullable<Uuid>,
        date -> Timestamptz,
        tool -> Nullable<Text>,
        automatic -> Bool,
        confidence -> Float8,
        old_state -> Nullable<Jsonb>,
        new_state -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    expense_transactions (id) {
        id -> Uuid,
        expense_id -> Uuid,
        account_id -> Uuid,
        idx -> Int4,
        date -> Timestamptz,
        booking_date -> Nullable<Timestamptz>,
        amount -> Nullable<Int4>,
        fraction -> Nullable<Float8>,
        comments -> Text,
        statement -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    expenses (id) {
        id -> Uuid,
        title -> Text,
        store -> Text,
        description -> Text,
        comments -> Text,
        reference_amount -> Nullable<Int4>,
        accrual_start -> Timestamptz,
        accrual_end -> Timestamptz,
        is_template -> Bool,
        is_tax_relevant -> Bool,
        is_unchecked -> Bool,
        is_incomplete -> Bool,
        is_extraordinary -> Bool,
        receipts -> Jsonb,
        positions -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    periodic_rules (id) {
        id -> Uuid,
        user_id -> Uuid,
        title -> Text,
        template_id -> Uuid,
        period -> Text,
        title_replacement -> Nullable<Text>,
        amount_replacement -> Nullable<Int4>,
        last_creation -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    seen_deliveries (id) {
        id -> Uuid,
        user_id -> Uuid,
        account_id -> Uuid,
        date -> Timestamptz,
        amount -> Int4,
        statement -> Text,
        expense_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    users (id) {
        id -> Uuid,
        name -> Text,
        full_name -> Text,
        password_hash -> Text,
    }
}

diesel::joinable!(accounts -> users (user_id));
diesel::joinable!(automation_rules -> accounts (filter_account_id));
diesel::joinable!(automation_rules -> expenses (template_id));
diesel::joinable!(automation_rules -> users (user_id));
diesel::joinable!(balances -> accounts (account_id));
diesel::joinable!(expense_categorizations -> categories (category_id));
diesel::joinable!(expense_categorizations -> expenses (expense_id));
diesel::joinable!(expense_changes -> users (user_id));
diesel::joinable!(expense_transactions -> accounts (account_id));
diesel::joinable!(expense_transactions -> expenses (expense_id));
diesel::joinable!(periodic_rules -> expenses (template_id));
diesel::joinable!(periodic_rules -> users (user_id));
diesel::joinable!(seen_deliveries -> accounts (account_id));
diesel::joinable!(seen_deliveries -> expenses (expense_id));
diesel::joinable!(seen_deliveries -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    automation_rules,
    balances,
    categories,
    expense_categorizations,
    expense_changes,
    expense_transactions,
    expenses,
    periodic_rules,
    seen_deliveries,
    users,
);
