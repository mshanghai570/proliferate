use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRetrievalStatus {
    pub enabled: bool,
    pub indexed_files: usize,
    pub chunks: usize,
    pub skipped_files: usize,
    pub truncated: bool,
    pub indexed_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchProjectContextRequest {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContextExcerpt {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content_hash: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContextSearchResponse {
    pub workspace_id: String,
    pub status: ProjectRetrievalStatus,
    pub excerpts: Vec<ProjectContextExcerpt>,
}
