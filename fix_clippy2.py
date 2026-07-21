import re

for path in ['tests/auth_security_test.rs', 'tests/password_reset_api.rs']:
    try:
        with open(path, 'r') as f:
            content = f.read()
        
        # Replace:
        # let mut config = Config::default();
        # config.security = test_security_config();
        # With:
        # let mut config = Config { security: test_security_config(), ..Default::default() };
        content = content.replace(
            "    let mut config = Config::default();\n    config.security = test_security_config();",
            "    let mut config = Config { security: test_security_config(), ..Default::default() };"
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

# tests/integration_test.rs
path = 'tests/integration_test.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
    
    # replace identity_users_real::create_user(&provider, ... with identity_users_real::create_user(provider, ...
    content = content.replace("(&provider, ", "(provider, ")
    content = content.replace("identity_roles_real::list_roles(&provider).await", "identity_roles_real::list_roles(provider).await")
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

