import re

path = 'tests/security_test.rs'
with open(path, 'r') as f:
    content = f.read()

content = content.replace(
'''    let expected = serde_json::json!({
        "error": "invalid credentials",
        "code": "unauthorized"
    });''',
'''    let expected = serde_json::json!({
        "error": "Invalid username or password.",
        "code": "invalid_credentials"
    });'''
)

with open(path, 'w') as f:
    f.write(content)
