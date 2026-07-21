import re

path = 'tests/integration_test.rs'
with open(path, 'r') as f:
    content = f.read()

# Fix remaining `&pool` being passed to mock functions (accounting for newlines and whitespace)
content = re.sub(r'&\s*pool\s*,', '&provider,', content)

# Fix remaining `pool: &SqlitePool` in mock signatures
content = content.replace(
    'pool: &SqlitePool',
    'provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>'
)

# Fix revoke_token call which requires extra arguments now.
content = content.replace(
    'nx9_auth::security::tokens::revoke_token(&provider, &token.id, /* Option<&str> */, /* Option<&str> */, /* Option<&str> */).await.unwrap();',
    'provider.tokens().revoke(&token.id).await.unwrap();'
)
# Just in case my previous attempt didn't add the comments
content = content.replace(
    'nx9_auth::security::tokens::revoke_token(&provider, &token.id).await.unwrap();',
    'provider.tokens().revoke(&token.id).await.unwrap();'
)

with open(path, 'w') as f:
    f.write(content)
