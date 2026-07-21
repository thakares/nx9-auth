CREATE TABLE IF NOT EXISTS roles (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    description TEXT
);
