CREATE TABLE installed_extensions (
    namespace TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK(length(sha256) = 64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
    state TEXT NOT NULL CHECK(state IN ('installing', 'installed', 'removing', 'error')),
    error_code TEXT CHECK(error_code IN ('interrupted_install', 'package_missing', 'package_invalid')),
    removal_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (namespace, name, version),
    CHECK((state = 'error') = (error_code IS NOT NULL)),
    CHECK((state = 'removing') = (removal_id IS NOT NULL))
);
