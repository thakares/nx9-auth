//! Runtime metrics and operational counters.

/// Lightweight runtime metrics storage used by the runtime layer.
#[derive(Debug, Clone, Default)]
pub struct RuntimeMetrics {
    requests_total: u64,
    errors_total: u64,
    active_workers: usize,
}

impl RuntimeMetrics {
    /// Create a new metrics container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a completed request.
    pub fn record_request(&mut self) {
        self.requests_total += 1;
    }

    /// Record a runtime error.
    pub fn record_error(&mut self) {
        self.errors_total += 1;
    }

    /// Update the current number of active workers.
    pub fn set_active_workers(&mut self, count: usize) {
        self.active_workers = count;
    }

    /// Return the total number of processed requests.
    pub fn requests_total(&self) -> u64 {
        self.requests_total
    }

    /// Return the total number of runtime errors.
    pub fn errors_total(&self) -> u64 {
        self.errors_total
    }

    /// Return the current worker count.
    pub fn active_workers(&self) -> usize {
        self.active_workers
    }
}
