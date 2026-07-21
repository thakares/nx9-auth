import re
import glob

# 1. ServerConfig missing fields
for path in ['tests/security_test.rs', 'tests/integration_test.rs']:
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = re.sub(
            r'server:\s*nx9_auth::config::ServerConfig\s*\{\s*host:\s*"127\.0\.0\.1"\.into\(\),\s*port:\s*8080,?\s*\}',
            r'server: nx9_auth::config::ServerConfig {\n            host: "127.0.0.1".into(),\n            port: 8080,\n            cookie_secure: false,\n            production: false,\n        }',
            content
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

# 2. cli_test.rs
path = 'tests/cli_test.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
        
    content = content.replace('nx9_auth::db::run_sqlite_migrations(&provider)', 'nx9_auth::db::run_migrations(&pool)')
    content = content.replace('nx9_auth::db::run_sqlite_migrations(&pool)', 'nx9_auth::db::run_migrations(&pool)')
    
    # username_exists
    content = content.replace(
        'nx9_auth::db::repository::users::username_exists(\n        &provider,',
        'provider.users().username_exists('
    )
    content = content.replace(
        'nx9_auth::db::repository::users::username_exists(&provider,',
        'provider.users().username_exists('
    )
    
    # Ensure provider is instantiated for `provider.users().username_exists` in cli_test.rs if needed.
    # Actually, cli_test.rs does NOT have a provider. It has a pool!
    # Wait, `provider` was in the compile error: `tests/cli_test.rs:160: &provider not found in this scope`.
    # Let me just provide a provider if pool is there!
    content = content.replace(
        'let admin_exists = provider.users().username_exists(',
        'let provider = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));\n    let admin_exists = provider.users().username_exists('
    )
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

# 3. migration_compatibility.rs
path = 'tests/migration_compatibility.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
        
    content = content.replace(
        'let admin_role = provider.roles().find_by_name(',
        'let provider = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));\n    let admin_role = provider.roles().find_by_name('
    )
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

# 4. src/main.rs - E0308 PostgresProvider::new(pool) where pool is SqlitePool
path = 'src/main.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
        
    # Replace the conflicting conditional logic to just use SqliteProvider for now.
    # Or properly cfg(feature).
    # Since we must keep SQLite exclusively per instructions:
    content = re.sub(
        r'#\[cfg\(feature = "postgres"\)\].*?let provider.*?SqliteProvider::new\(pool\)\)\s*\};',
        r'let provider: std::sync::Arc<dyn db::provider::DatabaseProvider> = std::sync::Arc::new(db::provider::SqliteProvider::new(pool));',
        content,
        flags=re.DOTALL
    )
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass
