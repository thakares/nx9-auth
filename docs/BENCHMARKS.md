# nx9-auth Performance Benchmarks

This document records the performance profiles, throughput (QPS), and latency percentiles for critical authentication pathways in `nx9-auth`.

These benchmarks were measured using the embedded `bench` binary (`cargo run --bin bench` or `cargo run --release --bin bench`).

---

## Benchmark Results

### 1. Password Verification (Argon2id KDF)

Password verification is CPU-bound and deliberately computationally heavy to protect against offline brute-force attacks.

#### Production Configuration (64 MiB memory, 3 iterations, 1 parallelism)
- **Requests/sec**: `0.63` (highly secure)
- **P50 Latency**: `1578.48 ms`
- **P95 Latency**: `1600.56 ms`
- **P99 Latency**: `1600.56 ms`

#### Fast/Test Configuration (4 MiB memory, 1 iteration, 1 parallelism)
- **Requests/sec**: `32.07`
- **P50 Latency**: `31.15 ms`
- **P95 Latency**: `31.56 ms`
- **P99 Latency**: `31.73 ms`

---

## 2. Session and Token Validation (BLAKE3 Hashing + SQLite)

Session and PAT validation do not run the heavy Argon2id algorithm. Instead, they use BLAKE3 hashing and look up the session/token in SQLite, updating the `last_seen_at`/`last_used_at` timestamps. These paths are extremely fast.

### Session Validation (Cookie authentication)
- **Requests/sec**: `9,259.47`
- **P50 Latency**: `0.10 ms`
- **P95 Latency**: `0.16 ms`
- **P99 Latency**: `0.24 ms`

### Personal Access Token (PAT) Validation
- **Requests/sec**: `9,500.90`
- **P50 Latency**: `0.09 ms`
- **P95 Latency**: `0.16 ms`
- **P99 Latency**: `0.21 ms`

---

## Key Takeaways

1. **Security & Latency Tradeoff**: The password login pathway is computationally heavy (~1.6 seconds) to guarantee state-of-the-art protection against hardware brute-force attacks.
2. **Ultra-Fast Session/PAT Verification**: Once a user is authenticated, microservice session and PAT validation checks are extremely cheap (~0.1ms), enabling low-overhead checks on every inbound API call for consumer systems like BZOD.
