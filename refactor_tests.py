import os
import glob

def refactor_test_file(path):
    with open(path, 'r') as f:
        content = f.read()

    # 1. Update setup_test_db signature
    content = content.replace(
        'async fn setup_test_db() -> (sqlx::SqlitePool, String)',
        'async fn setup_test_db() -> (std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>, sqlx::SqlitePool, String)'
    )
    # Fix the ones that already got halfway replaced
    content = content.replace(
        'async fn setup_test_db() -> (std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>, String)',
        'async fn setup_test_db() -> (std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>, sqlx::SqlitePool, String)'
    )

    # 2. Update setup_test_db body return
    content = content.replace(
        '(pool, db_path)\n}',
        'let provider = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));\n    (provider, pool, db_path)\n}'
    )
    content = content.replace(
        '(provider, db_path)\n}',
        '(provider, pool, db_path)\n}'
    )

    # 3. Update calls to setup_test_db
    content = content.replace(
        'let (pool, db_path) = setup_test_db().await;',
        'let (provider, pool, db_path) = setup_test_db().await;'
    )
    content = content.replace(
        'let (provider, db_path) = setup_test_db().await;',
        'let (provider, pool, db_path) = setup_test_db().await;'
    )

    # 4. AppState::new(pool...) -> AppState::new(provider...)
    content = content.replace('AppState::new(pool.clone(),', 'AppState::new(provider.clone(),')
    content = content.replace('AppState::new(pool,', 'AppState::new(provider.clone(),')
    # Unwind any previously wrapped AppState::new
    content = content.replace(
        'AppState::new(std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone())),',
        'AppState::new(provider.clone(),'
    )
    content = content.replace(
        'AppState::new(std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool)),',
        'AppState::new(provider.clone(),'
    )

    # 5. Fix all API calls passing &pool to pass &provider instead
    # The safest way is to just replace all `(&pool,` with `(&provider,` in the tests block.
    # But wait, `pool.fetch_one` or `pool.begin()` are `&pool` or `pool.`. So `(&pool, ` is safe.
    content = content.replace('(&pool,', '(&provider,')
    content = content.replace('(&pool)', '(&provider)')
    
    # Also fix explicit helper module calls in integration tests (the mocks)
    content = content.replace('identity_users::create_user(&pool,', 'identity_users::create_user(&provider,')
    
    # 6. Fix ServerConfig initialization missing fields in integration_test.rs
    content = content.replace(
        'server: nx9_auth::config::ServerConfig {\n            host: "127.0.0.1".into(),\n            port: 8080,\n        }',
        'server: nx9_auth::config::ServerConfig {\n            host: "127.0.0.1".into(),\n            port: 8080,\n            cookie_secure: false,\n            production: false,\n        }'
    )
    
    # Fix role_repo and token_repo direct calls in integration_test.rs
    content = content.replace('role_repo::list_for_user(&provider,', 'nx9_auth::identity::roles::list_user_roles(&provider,')
    content = content.replace('role_repo::find_by_name(&provider,', 'nx9_auth::identity::roles::find_role_by_name(&provider,') # if find_role_by_name doesn't exist, we'll fix it later
    # token_repo::revoke(&mut tx, &token.id)
    content = content.replace('token_repo::revoke(&mut tx, &token.id).await.unwrap();', 'nx9_auth::security::tokens::revoke_token(&provider, &token.id).await.unwrap();')
    content = content.replace('role_repo::remove_from_user(&mut tx, &user.id, &role.id).await.unwrap();', 'nx9_auth::identity::roles::remove_role(&provider, &user.id, &role.name).await.unwrap();')

    with open(path, 'w') as f:
        f.write(content)

for f in glob.glob('tests/*.rs'):
    if f != 'tests/migration_compatibility.rs':
        refactor_test_file(f)
