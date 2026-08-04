use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub id: i64,
    pub prompt_summary: String,
    pub commands: Vec<String>,
    pub code_snippets: Vec<CodeSnippet>,
    pub project_path: String,
    pub git_repo: Option<String>,
    pub is_pinned: bool,
    pub timestamp: i64,
}

impl Solution {
    pub fn formatted_date(&self) -> String {
        use chrono::TimeZone;
        if let Some(dt) = chrono::Utc.timestamp_opt(self.timestamp, 0).single() {
            dt.format("%Y-%m-%d %H:%M").to_string()
        } else {
            "Unknown Date".to_string()
        }
    }
}
