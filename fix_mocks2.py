import re
import glob

path = 'tests/integration_test.rs'
with open(path, 'r') as f:
    content = f.read()

content = content.replace(
    'let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));\n',
    ''
)
content = content.replace(
    '        let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));\n',
    ''
)

with open(path, 'w') as f:
    f.write(content)
