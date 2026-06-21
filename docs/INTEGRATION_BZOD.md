# BZOD Consumer Integration Guide

This guide details how BZOD consumes authentication and authorization services provided by `nx9-auth`.

---

## 1. Authentication

To authenticate a user and establish a session, send a `POST` request to `/api/v1/auth/login`.

### Request
- **Method**: `POST`
- **Path**: `/api/v1/auth/login`
- **Headers**: `Content-Type: application/json`
- **Payload**:
  ```json
  {
    "username": "admin",
    "password": "super_secure_password"
  }
  ```

### Response
- **Status**: `200 OK`
- **Headers**: `Set-Cookie: nx9_session=<session_token>; HttpOnly; Secure; SameSite=Lax; Path=/`
- **Payload**:
  ```json
  {
    "success": true
  }
  ```

---

## 2. Session Validation

To validate an existing session cookie and get the authenticated user's profile, roles, and permissions, make a `GET` request to `/api/v1/auth/me`.

### Request
- **Method**: `GET`
- **Path**: `/api/v1/auth/me`
- **Headers**: Include the `nx9_session` cookie in the request.

### Response
- **Status**: `200 OK`
- **Payload**:
  ```json
  {
    "user": {
      "id": "e4d3a2b1-5c6d-7e8f-9a0b-1c2d3e4f5a6b",
      "username": "admin",
      "status": "active",
      "last_login_at": "2026-06-21T18:09:13Z",
      "created_at": "2026-06-20T12:00:00Z"
    },
    "roles": ["admin"],
    "permissions": ["users:create", "users:update", "users:delete", "tokens:create", "tokens:revoke"]
  }
  ```

---

## 3. Personal Access Token (PAT) Authentication

For programmatic API access (service-to-service or CLI usage), clients can authenticate using a Personal Access Token (PAT) passed in the `Authorization` header.

### Request
- **Headers**: `Authorization: Bearer nx9_pat_<64_hex_chars>`

For example:
```bash
curl -H "Authorization: Bearer nx9_pat_29b2fd8c34f0f089..." https://auth.nx9.local/api/v1/auth/me
```

---

## 4. Permissions Mapping

The following table maps BZOD features and features to their required `nx9-auth` permissions:

| BZOD Feature / Action | Required Permission | Description |
|---|---|---|
| Create link | `links:create` | Allows creating new shortened links |
| Delete link | `links:delete` | Allows deleting existing shortened links |
| View link stats | `links:stats` | Allows viewing redirection analytics and link statistics |
| Create user accounts | `users:create` | Allows administrative user creation |
| Modify user status | `users:update` | Allows enabling, disabling, or locking users |
| Delete user accounts | `users:delete` | Allows soft-deleting/disabling users |

---

## 5. Unified Error Payload

All `nx9-auth` errors return a unified JSON payload format mapping to standard HTTP status codes:

```json
{
  "error": "Reason for the error",
  "code": "error_code"
}
```

### Standard Status Codes & Codes Mapping

| HTTP Status | Code | Description | Example Error |
|---|---|---|---|
| `401 Unauthorized` | `unauthorized` | Credentials are invalid, or session/token is missing/expired | `{"error": "invalid credentials", "code": "unauthorized"}` |
| `403 Forbidden` | `forbidden` | Authenticated user lacks the required permission | `{"error": "insufficient permissions", "code": "forbidden"}` |
| `404 Not Found` | `not_found` | Resource does not exist | `{"error": "resource not found", "code": "not_found"}` |
| `409 Conflict` | `conflict` | Unique constraint violation (e.g. username taken) | `{"error": "conflict: username already taken", "code": "conflict"}` |
| `422 Unprocessable` | `invalid_input` | Request body or payload format is invalid | `{"error": "invalid input: username cannot be empty", "code": "invalid_input"}` |
| `429 Too Many Requests` | `rate_limited` | Rate limit threshold exceeded | `{"error": "too many requests", "code": "rate_limited"}` |
| `500 Internal Error` | `internal_error` | Database query failure or unexpected server error | `{"error": "internal error", "code": "internal_error"}` |
