# Authentication Security

nx9-auth implements an OWASP-aligned login flow.

## Login contract

```http
POST /api/v1/auth/login
Content-Type: application/json
Accept: application/json

{
  "username": "sunil",
  "password": "Password123!"
}
```

### Response (200)

```json
{
  "access_token": "<opaque session token>",
  "refresh_token": "<opaque refresh token>",
  "expires_in": 86400,
  "token_type": "Bearer",
  "user": {
    "id": "...",
    "username": "...",
    "status": "active",
    "roles": ["admin"],
    "permissions": ["users:create", "..."]
  }
}
```

Also sets an HttpOnly `nx9_session` cookie (same value as `access_token`).

### Failures

| Status | Meaning |
|--------|---------|
| 401 | Invalid username or password (non-enumerating) |
| 400 | Malformed request body |
| 429 | Rate limited |

There is **no GET login**. Query-string credentials are never accepted.

## Password handling

| Layer | Behavior |
|-------|----------|
| Transport | HTTPS in production (`cookie_secure` + reverse-proxy TLS) |
| Client | Sends plaintext password **only** in POST JSON body — never hashes client-side |
| Server | Argon2id PHC (`$argon2id$v=19$…`) with unique salt |
| Storage | Only password hashes — never plaintext |
| Logs | Never log password, tokens, cookies, or Authorization |

## Session security

- New session token on every successful login (rotation)
- Prior sessions and refresh tokens revoked on login (fixation mitigation)
- Idle TTL + absolute TTL
- Session token hashed (BLAKE3) at rest
- Refresh tokens hashed (BLAKE3) in `refresh_tokens` table

## SPA client

1. `POST /api/v1/auth/login` with JSON
2. Store `access_token` in `sessionStorage`
3. Send `Authorization: Bearer <access_token>` on subsequent requests
4. Browser may also store HttpOnly cookie automatically

The HTML login form uses `method="post"` so a native fallback cannot leak credentials into the URL.

## Production configuration

```toml
[server]
cookie_secure = true
production = true
```

- `production = true` refuses `cookie_secure = false`
- Enables `Strict-Transport-Security` when secure mode is on
- Terminate TLS (TLS 1.3 recommended) at a reverse proxy or load balancer

## Security headers

Every response includes:

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: no-referrer`
- `Content-Security-Policy: …`
- `Permissions-Policy: …`
- `Strict-Transport-Security` (when production/secure)

## Rate limiting

Login is rate-limited per IP with progressive lockout (see `security::rate_limit`).
