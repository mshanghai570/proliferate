CREATE TABLE user_mcp_servers (
    id TEXT NOT NULL PRIMARY KEY,
    server_name TEXT NOT NULL UNIQUE,
    transport TEXT NOT NULL CHECK(transport IN ('stdio', 'http')),
    description TEXT,
    stdio_command TEXT,
    stdio_args_json TEXT NOT NULL DEFAULT '[]',
    env_sealed TEXT,
    url TEXT,
    headers_sealed TEXT,
    enabled_user_default INTEGER CHECK(enabled_user_default IN (0, 1)),
    repository_overrides_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(transport = 'stdio' OR url IS NOT NULL),
    CHECK(transport <> 'stdio' OR stdio_command IS NOT NULL)
);
