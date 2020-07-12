use chrono::{DateTime, Utc};
use log::error;
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::error::Error;

pub fn log_error_and_500(e: Box<dyn Error>) -> Status {
    error!("{}", e);
    Status::InternalServerError
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortBy {
    pub direction: String, // ascending | descending
    pub column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub page: i64,
    pub rows_per_page: i64,
    pub needle: Option<String>,
    pub sort_by: Vec<SortBy>,
    pub filter_by: HashMap<String, Vec<String>>, // column -> needles
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse<T> {
    pub records: Vec<T>,
    pub filtered_record_count: i64,
    pub total_record_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub total_record_count: i64,
    pub filter_hints: HashMap<String, Vec<String>>, // lists of some possible filter values for some columns
}
