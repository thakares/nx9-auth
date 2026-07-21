import glob

def fix_auth_security_test():
    path = 'tests/auth_security_test.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = content.replace('nx9_auth::db::run_sqlite_migrations(&provider)', 'nx9_auth::db::run_migrations(&pool)')
        content = content.replace('nx9_auth::db::run_sqlite_migrations(&pool)', 'nx9_auth::db::run_migrations(&pool)')
        content = content.replace('nx9_auth::db::create_sqlite_pool', 'nx9_auth::db::create_pool')
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

def fix_password_reset_api():
    path = 'tests/password_reset_api.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = content.replace('nx9_auth::db::run_sqlite_migrations(&provider)', 'nx9_auth::db::run_migrations(&pool)')
        content = content.replace('nx9_auth::db::run_sqlite_migrations(&pool)', 'nx9_auth::db::run_migrations(&pool)')
        content = content.replace('nx9_auth::db::create_sqlite_pool', 'nx9_auth::db::create_pool')
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

def fix_integration_test():
    path = 'tests/integration_test.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
        
        # role_repo::find_by_name -> provider.roles().find_by_name
        content = content.replace(
            'nx9_auth::identity::roles::find_role_by_name(&provider, "admin").await.unwrap();',
            'provider.roles().find_by_name("admin").await.unwrap();'
        )
        content = content.replace(
            'nx9_auth::identity::roles::find_role_by_name(&provider, "viewer").await.unwrap();',
            'provider.roles().find_by_name("viewer").await.unwrap();'
        )
        # generic catch all if there are others
        import re
        content = re.sub(
            r'nx9_auth::identity::roles::find_role_by_name\(&provider,\s*([^)]+)\)',
            r'provider.roles().find_by_name(\1)',
            content
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

fix_auth_security_test()
fix_password_reset_api()
fix_integration_test()
