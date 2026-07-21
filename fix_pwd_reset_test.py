import re

path = 'tests/password_reset_api.rs'
with open(path, 'r') as f:
    content = f.read()

replacement = """
    let admin_role = state.provider.roles().find_by_name("admin").await.unwrap().unwrap();
    state.provider.roles().assign_to_user(&admin.id, &admin_role.id).await.unwrap();
    
    (state, db_path, admin.id)
"""

content = content.replace("    (state, db_path, admin.id)", replacement)

with open(path, 'w') as f:
    f.write(content)
