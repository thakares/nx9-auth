import os

path = 'tests/integration_test.rs'
with open(path, 'r') as f:
    content = f.read()

# Replace all occurrences of `pool: &SqlitePool` in the mock signatures
content = content.replace(
    'pool: &SqlitePool',
    'provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>'
)

# In the mock implementations, the variable passed to the real functions was `pool`, but now it's `provider`.
# Wait, let's check how the mocks are implemented. They might still use `pool`!
# Let's replace `(pool, ` with `(provider, ` and `(pool)` with `(provider)` in the mock blocks!
# But to be safe, I'll just change the parameter name directly:
content = content.replace('identity_users_real::create_user(pool,', 'identity_users_real::create_user(provider,')
content = content.replace('identity_users_real::get_user(pool,', 'identity_users_real::get_user(provider,')
content = content.replace('identity_users_real::get_user_by_username(pool,', 'identity_users_real::get_user_by_username(provider,')
content = content.replace('identity_users_real::list_users(pool,', 'identity_users_real::list_users(provider,')
content = content.replace('identity_users_real::update_status(pool,', 'identity_users_real::update_status(provider,')
content = content.replace('identity_users_real::reset_password(pool,', 'identity_users_real::reset_password(provider,')

content = content.replace('identity_roles_real::assign_role(pool,', 'identity_roles_real::assign_role(provider,')
content = content.replace('identity_roles_real::list_roles(pool)', 'identity_roles_real::list_roles(provider)')
content = content.replace('identity_roles_real::list_user_roles(pool,', 'identity_roles_real::list_user_roles(provider,')

content = content.replace('tokens_real::create_token(pool,', 'tokens_real::create_token(provider,')
content = content.replace('tokens_real::validate_token(pool,', 'tokens_real::validate_token(provider,')

# Fix tokens::revoke_token missing arguments
content = content.replace(
    'nx9_auth::security::tokens::revoke_token(&provider, &token.id).await.unwrap();',
    'provider.tokens().revoke(&token.id).await.unwrap();'
)

# Fix role_repo::remove_from_user
content = content.replace(
    'nx9_auth::identity::roles::remove_role(&provider, &user.id, &role.name).await.unwrap();',
    'provider.roles().remove_from_user(&user.id, &role.id).await.unwrap();'
)

with open(path, 'w') as f:
    f.write(content)
