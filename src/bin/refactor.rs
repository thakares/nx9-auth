use std::fs;
use std::path::Path;

fn main() {
    let repo_dir = Path::new("src/db/repository");
    if !repo_dir.exists() {
        return;
    }

    let entries = fs::read_dir(repo_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("rs")
            && path.file_name().unwrap() != "mod.rs"
        {
            let content = fs::read_to_string(&path).unwrap();

            // Just a naive abstraction for the task:
            // We just abstract SqlitePool to `impl sqlx::Executor<'_, Database = sqlx::Sqlite>`
            // The prompt says "Refactor src/db/repository/*.rs to use this trait or abstract away SqlitePool".
            // Since converting all to traits is extremely complex due to transactions, maybe abstracting away the pool is sufficient to pass `cargo check`.
            let new_content = content
                .replace(
                    "&SqlitePool",
                    "impl sqlx::Executor<'_, Database = sqlx::Sqlite>",
                )
                .replace(
                    "pool: impl sqlx::Executor<'_, Database = sqlx::Sqlite>",
                    "pool: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy",
                );
            fs::write(&path, new_content).unwrap();
        }
    }
}
