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

## Audit Logging Security

Audit logs record critical identity lifecycle events while strictly redacting sensitive fields:
- **Recorded Events**: Login success/failure, logout, password change, user creation/deletion, API token issuance/revocation, role/permission assignments.
- **Redaction Rules**: Plaintext passwords, password hashes, bearer tokens, refresh tokens, session secrets, and `Authorization` headers are **never** logged under any circumstances.

## Rate Limiting & Protection

- **Progressive Lockout**: Progressive rate limiting protects sensitive endpoints (`/auth/login`, `/users/{id}/reset-password`, `/tokens`) against brute-force and credential-stuffing attacks.
