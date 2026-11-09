use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMcpTransport {
    Http,
    Stdio,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMcpServerMetadata {
    pub name: String,
    pub enabled: bool,
    pub transport: ProjectMcpTransport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMcpInventory {
    pub servers: Vec<ProjectMcpServerMetadata>,
    pub encryption_available: bool,
}
