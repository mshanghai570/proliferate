CREATE TABLE project_mcp_servers (
    repo_root_id TEXT NOT NULL REFERENCES repo_roots(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    transport TEXT NOT NULL CHECK (transport IN ('http', 'stdio')),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    ciphertext TEXT NOT NULL,
    PRIMARY KEY (repo_root_id, name)
);
