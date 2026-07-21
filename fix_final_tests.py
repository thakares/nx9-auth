import re

def fix_password_reset_api():
    path = 'tests/password_reset_api.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = content.replace(
            'let pool = state.pool.clone();',
            'let pool = state.provider.clone();'
        )
        content = content.replace(
            'tokens::create_token(\n        &pool,',
            'tokens::create_token(\n        &state.provider,'
        )
        content = content.replace(
            'tokens::create_token(&pool,',
            'tokens::create_token(&state.provider,'
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

def fix_security_test():
    path = 'tests/security_test.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        # Regex to fix ServerConfig initialization robustly
        content = re.sub(
            r'server:\s*nx9_auth::config::ServerConfig\s*\{\s*host:\s*"127\.0\.0\.1"\.into\(\),\s*port:\s*8080,\s*\}',
            r'server: nx9_auth::config::ServerConfig {\n            host: "127.0.0.1".into(),\n            port: 8080,\n            cookie_secure: false,\n            production: false,\n        }',
            content
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

fix_password_reset_api()
fix_security_test()
