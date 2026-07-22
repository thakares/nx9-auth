//! Prioritized, idempotent shutdown hooks.

use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShutdownPriority {
    First = 0,
    Normal = 1,
    Last = 2,
}

#[async_trait::async_trait]
pub trait ShutdownHook: Send + Sync {
    fn name(&self) -> &'static str;

    fn priority(&self) -> ShutdownPriority {
        ShutdownPriority::Normal
    }

    async fn shutdown(&self) -> Result<()>;
}

pub struct HookRegistry {
    hooks: Vec<Box<dyn ShutdownHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Box<dyn ShutdownHook>) {
        tracing::debug!(hook = hook.name(), priority = ?hook.priority(), "shutdown hook registered");
        self.hooks.push(hook);
    }

    pub async fn execute_all(&self) {
        if self.hooks.is_empty() {
            return;
        }

        let mut indices: Vec<usize> = (0..self.hooks.len()).collect();
        indices.sort_by_key(|&i| self.hooks[i].priority());

        for i in indices {
            let hook = &self.hooks[i];
            let start = std::time::Instant::now();
            tracing::info!(hook = hook.name(), priority = ?hook.priority(), "executing shutdown hook");
            match hook.shutdown().await {
                Ok(()) => {
                    tracing::info!(
                        hook = hook.name(),
                        duration_ms = start.elapsed().as_millis(),
                        "shutdown hook completed successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        hook = hook.name(),
                        duration_ms = start.elapsed().as_millis(),
                        error = %e,
                        "shutdown hook failed"
                    );
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
