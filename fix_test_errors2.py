import re

path = 'tests/integration_test.rs'
with open(path, 'r') as f:
    content = f.read()

content = content.replace(
'''        "admin_dashboard",
        "S3cur3#P@ssw0rd!",
        None, None, None
    ).await.unwrap();''',
'''        "admin_dashboard",
        "S3cur3#P@ssw0rd!"
    ).await.unwrap();'''
)

with open(path, 'w') as f:
    f.write(content)
