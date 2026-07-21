import glob

def fix_file(path):
    with open(path, 'r') as f:
        content = f.read()

    # 1. security_test.rs run_migrations
    content = content.replace('db::run_migrations(&provider)', 'nx9_auth::db::run_migrations(&pool)')
    content = content.replace('nx9_auth::db::run_sqlite_migrations(&pool)', 'nx9_auth::db::run_migrations(&pool)')
    
    # Fix setup_test_db returning provider instead of pool? Wait! `setup_test_db` in `tests/security_test.rs` currently returns `(provider, db_path)`.
    # Let me make it return `(provider, pool, db_path)` like integration_test.rs did.
    content = content.replace(
        'let provider = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool));\n    (provider, db_path)',
        'let provider = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));\n    (provider, pool, db_path)'
    )
    content = content.replace(
        'let (provider, db_path) = setup_test_db().await;',
        'let (provider, pool, db_path) = setup_test_db().await;'
    )

    # 2. security_test.rs user_repo
    content = content.replace('user_repo::find_by_username(&provider,', 'provider.users().find_by_username(')
    content = content.replace('user_repo::find_by_id(&provider,', 'provider.users().find_by_id(')
    
    # role_repo
    content = content.replace('role_repo::assign_role(&provider,', 'nx9_auth::identity::roles::assign_role(&provider,')
    
    # 3. security_test.rs ServerConfig
    content = content.replace(
        'server: nx9_auth::config::ServerConfig {\n            host: "127.0.0.1".into(),\n            port: 8080,\n        }',
        'server: nx9_auth::config::ServerConfig {\n            host: "127.0.0.1".into(),\n            port: 8080,\n            cookie_secure: false,\n            production: false,\n        }'
    )
    
    # 4. cli_test.rs create_sqlite_pool
    content = content.replace('nx9_auth::db::create_sqlite_pool', 'nx9_auth::db::create_pool')
    
    with open(path, 'w') as f:
        f.write(content)

for f in glob.glob('tests/*.rs'):
    fix_file(f)
