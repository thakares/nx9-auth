//! Background worker management using `tokio::task::JoinSet`.

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use tokio::task::JoinSet;

use super::ShutdownCoordinator;

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

    pub async fn drain_all(&mut self) {
        for group in self.groups.values_mut() {
            group.abort_all();
            while group.tasks.join_next().await.is_some() {}
        }
    }

    /// Shut down all worker groups concurrently under a single global deadline,
    /// while observing live forced shutdown escalation.
    pub async fn shutdown_all_with_coordinator(
        &mut self,
        timeout: Duration,
        coordinator: Option<&ShutdownCoordinator>,
    ) {
        let active = self.active_tasks();
        if active == 0 {
            return;
        }

        tracing::info!(
            active_tasks = active,
            timeout_secs = timeout.as_secs(),
            "shutting down background worker task groups under global deadline"
        );

        let is_already_forced = coordinator.map(|c| c.is_forced()).unwrap_or(false);
        if is_already_forced {
            tracing::warn!("forced shutdown active; aborting all worker tasks immediately");
            self.drain_all().await;
            return;
        }

        let groups = std::mem::take(&mut self.groups);
        let mut group_joiners = JoinSet::new();
        let mut group_map = HashMap::new();

        for (name, mut group) in groups {
            group_joiners.spawn(async move {
                while group.tasks.join_next().await.is_some() {}
                (name, group)
            });
        }

        let join_all_fut = async {
            while let Some(res) = group_joiners.join_next().await {
                if let Ok((name, group)) = res {
                    group_map.insert(name, group);
                }
            }
        };

        let forced_fut = async {
            if let Some(coord) = coordinator {
                coord.forced_cancelled().await;
            } else {
                std::future::pending::<()>().await;
            }
        };

        tokio::select! {
            _ = join_all_fut => {
                tracing::info!("all worker task groups shut down cleanly");
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("global worker shutdown timeout expired; aborting remaining tasks");
                group_joiners.abort_all();
                while let Some(res) = group_joiners.join_next().await {
                    if let Ok((name, mut group)) = res {
                        group.abort_all();
                        group_map.insert(name, group);
                    }
                }
            }
            _ = forced_fut => {
                tracing::warn!("live forced shutdown escalation received during worker wait; aborting remaining tasks immediately");
                group_joiners.abort_all();
                while let Some(res) = group_joiners.join_next().await {
                    if let Ok((name, mut group)) = res {
                        group.abort_all();
                        group_map.insert(name, group);
                    }
                }
            }
        }

        for group in group_map.values_mut() {
            if !group.is_empty() {
                group.abort_all();
                while group.tasks.join_next().await.is_some() {}
            }
        }

        self.groups = group_map;
    }

    pub async fn shutdown_all(&mut self, timeout: Duration) {
        self.shutdown_all_with_coordinator(timeout, None).await;
    }
}

impl Default for WorkerManager {
    fn default() -> Self {
        Self::new()
    }
}
