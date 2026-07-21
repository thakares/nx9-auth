import re

def fix_integration_test():
    path = 'tests/integration_test.rs'
    try:
        with open(path, 'r') as f:
            content = f.read()
            
        content = re.sub(
            r'role_repo::remove_from_user\(&mut tx, &user\.id, &role\.id\)',
            r'provider.roles().remove_from_user(&user.id, &role.id)',
            content
        )
        
        with open(path, 'w') as f:
            f.write(content)
    except FileNotFoundError:
        pass

fix_integration_test()
