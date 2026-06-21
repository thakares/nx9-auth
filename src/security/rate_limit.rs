use std::{
    collections::VecDeque,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;

use crate::error::AppError;

/// Per-IP tracking state.
#[derive(Debug)]
struct IpState {
    /// Failure timestamps within the current window.
    window: VecDeque<Instant>,
    /// Number of times this IP has been locked out (escalation counter).
    lockout_count: u32,
    /// When the current lockout expires. `None` if not locked.
    locked_until: Option<Instant>,
}

impl IpState {
    fn new() -> Self {
        Self {
            window: VecDeque::new(),
            lockout_count: 0,
            locked_until: None,
        }
    }
}

/// In-memory escalating rate limiter for login attempts.
///
/// Policy:
/// - Track failures per IP in a 15-minute sliding window.
/// - After 5 failures → lock for 15 minutes (level 1).
/// - After another 5 failures post-unlock → lock for 1 hour (level 2).
/// - After another 5 failures post-unlock → lock for 24 hours (level 3+).
///
/// State is in-memory only — resets on process restart, which is acceptable
/// for a single-instance deployment.
#[derive(Debug)]
pub struct RateLimiter {
    state: DashMap<IpAddr, IpState>,
    /// Window for failure counting.
    window: Duration,
    /// Max failures per window before lockout.
    max_failures: u32,
}

impl RateLimiter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: DashMap::new(),
            window: Duration::from_secs(15 * 60),
            max_failures: 5,
        })
    }

    /// Calculate lockout duration based on escalation level.
    fn lockout_duration(level: u32) -> Duration {
        match level {
            1 => Duration::from_secs(15 * 60),      // 15 minutes
            2 => Duration::from_secs(60 * 60),      // 1 hour
            _ => Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }

    /// Check if the given IP is currently allowed to attempt a login.
    ///
    /// Returns `Err(AppError::RateLimited)` if the IP is locked out.
    pub fn check(&self, ip: IpAddr) -> Result<(), AppError> {
        let state = self.state.get(&ip);
        if let Some(s) = state {
            if let Some(until) = s.locked_until {
                if Instant::now() < until {
                    return Err(AppError::RateLimited);
                }
            }
        }
        Ok(())
    }

    /// Record a failed login attempt for an IP.
    ///
    /// Triggers lockout if the failure threshold is reached.
    pub fn record_failure(&self, ip: IpAddr) {
        let mut s = self.state.entry(ip).or_insert_with(IpState::new);
        let now = Instant::now();

        // Clear the lockout if it has expired
        if let Some(until) = s.locked_until {
            if now >= until {
                s.locked_until = None;
            }
        }

        // Prune old failures outside the window
        let cutoff = now - self.window;
        while s.window.front().is_some_and(|&t| t < cutoff) {
            s.window.pop_front();
        }

        s.window.push_back(now);

        if s.window.len() >= self.max_failures as usize {
            s.lockout_count += 1;
            let duration = Self::lockout_duration(s.lockout_count);
            s.locked_until = Some(now + duration);
            s.window.clear();

            tracing::warn!(
                ip = %ip,
                lockout_count = s.lockout_count,
                duration_secs = duration.as_secs(),
                "login rate limit triggered"
            );
        }
    }

    /// Record a successful login — clear failure history for this IP.
    pub fn record_success(&self, ip: IpAddr) {
        if let Some(mut s) = self.state.get_mut(&ip) {
            s.window.clear();
            s.locked_until = None;
            // Do NOT reset lockout_count — escalation persists across successful logins
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            state: DashMap::new(),
            window: Duration::from_secs(15 * 60),
            max_failures: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_rate_limiter() {
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limiter = RateLimiter {
            state: DashMap::new(),
            window: Duration::from_secs(60),
            max_failures: 3,
        };

        // Initially OK
        assert!(limiter.check(ip).is_ok());

        // First failure
        limiter.record_failure(ip);
        assert!(limiter.check(ip).is_ok());

        // Second failure
        limiter.record_failure(ip);
        assert!(limiter.check(ip).is_ok());

        // Third failure -> should trigger lockout
        limiter.record_failure(ip);
        assert!(limiter.check(ip).is_err());

        // Clear via success
        limiter.record_success(ip);
        assert!(limiter.check(ip).is_ok());
    }

    #[test]
    fn test_lockout_escalation() {
        assert_eq!(
            RateLimiter::lockout_duration(1),
            Duration::from_secs(15 * 60)
        );
        assert_eq!(
            RateLimiter::lockout_duration(2),
            Duration::from_secs(60 * 60)
        );
        assert_eq!(
            RateLimiter::lockout_duration(3),
            Duration::from_secs(24 * 60 * 60)
        );
    }
}
