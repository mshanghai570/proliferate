#[derive(Debug, Default, Clone)]
pub struct ProjectRetrievalStatus {
    pub enabled: bool,
    pub indexed_files: usize,
    pub chunks: usize,
    pub skipped_files: usize,
    pub truncated: bool,
    pub indexed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectContextExcerpt {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content_hash: String,
    pub text: String,
}

pub struct ProjectContextSearchResult {
    pub workspace_id: String,
    pub status: ProjectRetrievalStatus,
    pub excerpts: Vec<ProjectContextExcerpt>,
}
