import re

path = 'tests/integration_test.rs'
with open(path, 'r') as f:
    content = f.read()

# Fix the broken lines
content = content.replace("username, password, ,", "username, password, None, None, None,")
content = content.replace("user_id, status, ).await", "user_id, status, None, None, None).await")
content = content.replace("user_id, new_password, )", "user_id, new_password, None, None, None)")
content = content.replace("role_name, ).await", "role_name, None, None, None).await")
content = content.replace("name, cfg, ).await", "name, cfg, None, None, None).await")

# Fix the dashboard test specifically
content = content.replace(
'''        "admin_dashboard",
        "S3cur3#P@ssw0rd!",
        
    ).await.unwrap();''',
'''        "admin_dashboard",
        "S3cur3#P@ssw0rd!",
        None, None, None
    ).await.unwrap();'''
)

with open(path, 'w') as f:
    f.write(content)
