import re

path = 'src/db/repository/sqlite/applications.rs'
with open(path, 'r') as f:
    content = f.read()

# Fix create
content = content.replace(
    "RETURNING *",
    "RETURNING id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris"
)

# Fix find_by_slug
content = content.replace(
    '"SELECT * FROM applications WHERE slug = ?"',
    '"SELECT id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris FROM applications WHERE slug = ?"'
)

# Fix find_by_id
content = content.replace(
    '"SELECT * FROM applications WHERE id = ?"',
    '"SELECT id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris FROM applications WHERE id = ?"'
)

# Fix list
content = content.replace(
    '"SELECT * FROM applications WHERE tenant_id = ? ORDER BY name"',
    '"SELECT id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris FROM applications WHERE tenant_id = ? ORDER BY name"'
)

with open(path, 'w') as f:
    f.write(content)
