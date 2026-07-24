# NX9-Auth Security Policy & Controls

NX9-Auth is designed with a **security-first, privacy-first, zero-trust** architecture for self-hosted Identity & Access Management.

## Authentication & Password Security

- **POST-Only Authentication**: Login requests (`/api/v1/auth/login`) strictly accept JSON payloads via HTTP `POST`. GET login is rejected (HTTP 405) to prevent credentials from being exposed in URL query parameters, browser history, or server access logs.
- **Argon2id Password Hashing**: Passwords are hashed server-side using **Argon2id** (`$argon2id$v=19$m=19456,t=2,p=1$…`) with unique cryptographically random salts. Plaintext passwords are never stored, logged, or echoed.
- **Constant-Time Verification**: Password verification uses constant-time string comparisons (`subtle` / Argon2 verify) to eliminate timing side-channel attacks.
- **Non-Enumerating Error Messages**: Authentication failures return standardized error messages (`401 Unauthorized: Invalid username or password`) regardless of whether the user exists.

## HTTP & Session Security

- **Opaque Session & Refresh Tokens**: Tokens are generated via high-entropy `getrandom` buffers (`st_…`, `rt_…`, `pat_…`) and hashed using BLAKE3 at rest.
- **Cookie Security**: Session cookies (`nx9_session`) are set with `HttpOnly`, `SameSite=Lax`, and `Secure` (in production/HTTPS mode).
- **OWASP Security Headers**:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `Referrer-Policy: no-referrer`
  - `Cache-Control: no-store`
  - `Content-Security-Policy: default-src 'self' ...`
  - `Permissions-Policy: accelerometer=(), camera=(), geolocation=(), ...`
  - `Strict-Transport-Security: max-age=63072000; includeSubDomains` (when `cookie_secure` / production is enabled)

## Application Credentials & Client Authentication

- **Client ID & Client Secret**: Applications registered in NX9-Auth receive an immutable, server-generated `client_id` (`nx9_app_<32 lowercase hex chars>`) and high-entropy CSPRNG `client_secret` (`nx9_secret_<64 lowercase hex chars>`).
- **One-Time Secret Disclosure**: Plaintext client secrets are disclosed **exactly once** upon initial application creation and explicit secret rotation. Responses containing plaintext secrets include `Cache-Control: no-store` headers.
- **BLAKE3 Secret Hashing**: Only BLAKE3 cryptographic digests (`[u8; 32]`) are persisted in database records. Plaintext secrets are never stored, logged, serialized into GET/list API responses, or stored in browser persistence.
- **Constant-Time Raw Byte Verification**: Verification hashes supplied credentials to a 32-byte BLAKE3 digest and constant-time compares bytes against the stored 32-byte digest. To prevent timing side-channel attacks for unknown or uncredentialed applications, a dummy BLAKE3 comparison path is executed before returning non-enumerating `401 Unauthorized` errors.
- **Secret Rotation**: Administrator rotation immediately invalidates the previous client secret hash and generates a new secret.
- **Dedicated Permissions**: Application mutations (`create`, `update`, `rotate_secret`, `enable_disable`, `delete`) require the `applications:manage` permission.

## Tenant Reassignment Safety & Audit Atomicity

- **Option-A Tenant Isolation**: Users belong strictly to one tenant (`users.tenant_id`). Cross-tenant operations strictly check boundaries.
- **Transactional Atomicity**: Tenant reassignment (`reassign_user_tenant_with_audit`) executes inside a single database transaction. Database update (`users.tenant_id`) and audit log record insertion (`user.tenant_reassigned` with `from_tenant_id` and `to_tenant_id`) commit together. If audit log insertion fails, database changes roll back completely.
- **No-Op Reassignment**: Reassignment to the user's current tenant is a defined no-op that emits no audit log and avoids unnecessary database mutations.
- **Concurrency & Locking Protection**: PostgreSQL uses `SELECT ... FOR UPDATE` row locking; SQLite uses write transactions and conditional updates (`WHERE tenant_id = expected`).
- **Last-Admin Protection**: System administrator reassignment away from the default tenant is rejected if `count_admins() <= 1`.

## Application Membership Security & Audit Atomicity

- **Same-Tenant Enforcement**: Application membership requires user and application to share the exact same `tenant_id`. Cross-tenant membership is rejected.
- **Transactional Atomicity**: Application membership mutations (add, role update, enable/disable, remove) execute in single transactions with audit log insertions; audit failure automatically rolls back the membership change.
- **Strict Protocol Boundary**: Application authentication accepts only `client_id` + `client_secret`. Editable application slugs are never accepted as credential identities.

## Audit Logging Security & Export

Audit logs record critical identity lifecycle events while strictly redacting sensitive fields:
- **Recorded Events**: Login success/failure, logout, password change, user creation/deletion, tenant reassignment, API token issuance/revocation, application creation/secret rotation/membership modification, role/permission assignments.
- **Redaction Rules**: Plaintext passwords, password hashes, bearer tokens, refresh tokens, client secrets, client secret hashes, session secrets, and `Authorization` headers are **never** logged under any circumstances.
- **Bounded CSV Export**: Audit log CSV export uses server-side audit search APIs bounded to a maximum of 5,000 records matching currently active query filters, preserving exact tenant/RBAC restrictions and RFC-4180 field escaping.

## Rate Limiting & Protection

- **Progressive Lockout**: Progressive rate limiting protects sensitive endpoints (`/auth/login`, `/users/{id}/reset-password`, `/tokens`, `/applications/{id}/secret`) against brute-force and credential-stuffing attacks.
