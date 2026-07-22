#[cfg(feature = "sqlite")]
use nx9_auth::{
    config::SecurityConfig,
    db::{self, models::Tenant, provider::SqliteProvider},
    identity::users as identity_users,
    security::{passwords, sessions, tokens},
};
#[cfg(feature = "sqlite")]
use std::sync::Arc;
#[cfg(feature = "sqlite")]
use std::time::Instant;

#[cfg(feature = "sqlite")]
async fn setup_bench_db() -> (Arc<dyn nx9_auth::db::provider::DatabaseProvider>, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/bench_{}.db", db_id);
    let pool = db::create_pool(&db_path)
        .await
        .expect("Failed to create bench db");
    db::run_migrations(&pool)
        .await
        .expect("Failed to run bench migrations");
    let provider: Arc<dyn nx9_auth::db::provider::DatabaseProvider> =
        Arc::new(SqliteProvider::new(pool));
    (provider, db_path)
}

#[cfg(feature = "sqlite")]
fn print_stats(name: &str, mut durations: Vec<std::time::Duration>, count: usize) {
    durations.sort();
    let total_secs: f64 = durations.iter().map(|d| d.as_secs_f64()).sum();
    let qps = count as f64 / total_secs;

    let p50 = durations[count / 2];
    let p95 = durations[(count * 95) / 100];
    let p99 = durations[(count * 99) / 100];

    println!("{}:", name);
    println!("  Total ops:   {}", count);
    println!("  Requests/s:  {:.2}", qps);
    println!("  P50 latency: {:.2} ms", p50.as_secs_f64() * 1000.0);
    println!("  P95 latency: {:.2} ms", p95.as_secs_f64() * 1000.0);
    println!("  P99 latency: {:.2} ms", p99.as_secs_f64() * 1000.0);
    println!();
}

#[cfg(feature = "sqlite")]
#[tokio::main]
async fn main() {
    println!("Starting nx9-auth microbenchmarks...");
    let (provider, db_path) = setup_bench_db().await;

    // Production security config
    let sec_cfg = SecurityConfig {
        session_ttl_hours: 24,
        session_absolute_ttl_days: 30,
        token_ttl_days: 365,
        argon2_memory: 65536,  // Production: 64MiB
        argon2_iterations: 3,  // Production: 3 passes
        argon2_parallelism: 1, // Production: 1 thread
    };

    // Test security config (low cost to see algorithm overhead vs Argon2 KDF)
    let fast_sec_cfg = SecurityConfig {
        session_ttl_hours: 24,
        session_absolute_ttl_days: 30,
        token_ttl_days: 365,
        argon2_memory: 4096,
        argon2_iterations: 1,
        argon2_parallelism: 1,
    };

    // Create benchmark user
    let user = identity_users::create_user(
        &provider,
        &fast_sec_cfg,
        Tenant::DEFAULT_ID,
        "bench_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Password Verification Benchmark (Production Cost)
    // ─────────────────────────────────────────────────────────────────────────
    let prod_hash = passwords::hash_password("super_secure_passphrase_123", &sec_cfg).unwrap();
    let login_ops = 20;
    let mut login_durations = Vec::with_capacity(login_ops);

    for _ in 0..login_ops {
        let start = Instant::now();
        let ok = passwords::verify_password("super_secure_passphrase_123", &prod_hash).unwrap();
        assert!(ok);
        login_durations.push(start.elapsed());
    }
    print_stats(
        "Argon2id Password Verification (Production Config: 64MiB, 3 passes)",
        login_durations,
        login_ops,
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Password Verification Benchmark (Low Cost)
    // ─────────────────────────────────────────────────────────────────────────
    let fast_hash = passwords::hash_password("super_secure_passphrase_123", &fast_sec_cfg).unwrap();
    let fast_login_ops = 100;
    let mut fast_login_durations = Vec::with_capacity(fast_login_ops);

    for _ in 0..fast_login_ops {
        let start = Instant::now();
        let ok = passwords::verify_password("super_secure_passphrase_123", &fast_hash).unwrap();
        assert!(ok);
        fast_login_durations.push(start.elapsed());
    }
    print_stats(
        "Argon2id Password Verification (Test/Low Cost Config: 4MiB, 1 pass)",
        fast_login_durations,
        fast_login_ops,
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Session Validation Benchmark (BLAKE3 Hashing + SQLite)
    // ─────────────────────────────────────────────────────────────────────────
    let (_session, raw_token) = sessions::create_session(
        &provider,
        &user.id,
        Some("127.0.0.1"),
        Some("Bench Agent"),
        &fast_sec_cfg,
    )
    .await
    .unwrap();

    let session_ops = 2000;
    let mut session_durations = Vec::with_capacity(session_ops);

    for _ in 0..session_ops {
        let start = Instant::now();
        let validated = sessions::validate_session(&provider, &raw_token, &fast_sec_cfg)
            .await
            .unwrap();
        assert!(validated.is_some());
        session_durations.push(start.elapsed());
    }
    print_stats(
        "Session Validation (BLAKE3 + SQLite Touch)",
        session_durations,
        session_ops,
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 4. Personal Access Token (PAT) Verification Benchmark (BLAKE3 + SQLite)
    // ─────────────────────────────────────────────────────────────────────────
    let (_token, raw_pat) = tokens::create_token(
        &provider,
        &user.id,
        "bench-pat",
        &fast_sec_cfg,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let pat_ops = 2000;
    let mut pat_durations = Vec::with_capacity(pat_ops);

    for _ in 0..pat_ops {
        let start = Instant::now();
        let validated = tokens::validate_token(&provider, &raw_pat).await.unwrap();
        assert!(validated.is_some());
        pat_durations.push(start.elapsed());
    }
    print_stats(
        "PAT Validation (BLAKE3 + SQLite Touch)",
        pat_durations,
        pat_ops,
    );

    let _ = std::fs::remove_file(db_path);
}

#[cfg(not(feature = "sqlite"))]
fn main() {
    println!("Benchmark binary requires the 'sqlite' feature");
}
