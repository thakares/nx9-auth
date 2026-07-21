import re

def fix_migration_compat():
    path = 'tests/migration_compatibility.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = content.replace(
            'role_repo::find_by_name(&pool, "admin").await.unwrap();',
            'provider.roles().find_by_name("admin").await.unwrap();'
        )
        content = content.replace(
            'role_repo::find_by_name(&pool, "viewer").await.unwrap();',
            'provider.roles().find_by_name("viewer").await.unwrap();'
        )
        
        # Need to ensure `provider` is created!
        # find: `let pool = nx9_auth::db::create_pool(&db_path).await.unwrap();`
        content = content.replace(
            'let pool = nx9_auth::db::create_pool(&db_path).await.unwrap();',
            'let pool = nx9_auth::db::create_pool(&db_path).await.unwrap();\n    let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));'
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

def fix_password_reset_api():
    path = 'tests/password_reset_api.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = content.replace(
            'identity_users::create_user(\n        &provider,',
            'identity_users::create_user(\n        &state.provider,'
        )
        content = content.replace(
            'identity_users::create_user(&provider,',
            'identity_users::create_user(&state.provider,'
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

fix_migration_compat()
fix_password_reset_api()
