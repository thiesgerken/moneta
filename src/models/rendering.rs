
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpenseCategorization {
    pub category: RenderedCategory,
    // constraint: weight > 0
    pub weight: f64,
}
