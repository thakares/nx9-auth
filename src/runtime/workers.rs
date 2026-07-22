//! Background worker management using `tokio::task::JoinSet`.

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use tokio::task::JoinSet;

pub struct TaskGroup {
    name: String,
    tasks: JoinSet<()>,
}

impl TaskGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tasks: JoinSet::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks.spawn(future);
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn abort_all(&mut self) {
        self.tasks.abort_all();
    }

    pub async fn shutdown(&mut self, timeout: Duration) {
        if self.tasks.is_empty() {
            return;
        }

        let task_count = self.tasks.len();
        tracing::info!(
            group = %self.name,
            tasks = task_count,
            timeout_secs = timeout.as_secs(),
            "waiting for task group to complete"
        );

        let result = tokio::time::timeout(timeout, async {
            while self.tasks.join_next().await.is_some() {}
        })
        .await;

        if result.is_err() {
            let remaining = self.tasks.len();
            tracing::warn!(
                group = %self.name,
                remaining,
                "task group timed out, aborting remaining tasks"
            );
            self.tasks.abort_all();
            while self.tasks.join_next().await.is_some() {}
        }
    }
}

pub struct WorkerManager {
    groups: HashMap<String, TaskGroup>,
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    pub fn group(&mut self, name: &str) -> &mut TaskGroup {
        self.groups
            .entry(name.to_string())
            .or_insert_with(|| TaskGroup::new(name))
    }

    pub fn active_tasks(&self) -> usize {
        self.groups.values().map(|g| g.len()).sum()
    }

    pub fn abort_all(&mut self) {
        for group in self.groups.values_mut() {
            group.abort_all();
        }
    }

    pub async fn shutdown_all(&mut self, timeout: Duration) {
        for group in self.groups.values_mut() {
            group.shutdown(timeout).await;
        }
    }
}

impl Default for WorkerManager {
    fn default() -> Self {
        Self::new()
    }
}
