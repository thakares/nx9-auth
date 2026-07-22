use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use nx9_auth::config::Config;
use nx9_auth::runtime::{
    Application, HookRegistry, RuntimeState, ShutdownHook, ShutdownPriority, WorkerManager,
};

struct TestHook {
    name: &'static str,
    priority: ShutdownPriority,
    counter: Arc<AtomicUsize>,
    sequence: Arc<tokio::sync::Mutex<Vec<&'static str>>>,
}

#[async_trait::async_trait]
impl ShutdownHook for TestHook {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> ShutdownPriority {
        self.priority
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        let mut seq = self.sequence.lock().await;
        seq.push(self.name);
        Ok(())
    }
}

#[tokio::test]
async fn test_runtime_application_builder() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 0; // OS assigned port
    config.database.url = Some("sqlite::memory:".to_string());

    let mut app = Application::builder(config).build().await?;
    assert_eq!(app.state(), RuntimeState::Starting);

    app.perform_shutdown().await?;
    assert_eq!(app.state(), RuntimeState::Stopped);
    Ok(())
}

#[tokio::test]
async fn test_shutdown_hook_execution_order() {
    let counter = Arc::new(AtomicUsize::new(0));
    let sequence = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let hook_last = TestHook {
        name: "hook_last",
        priority: ShutdownPriority::Last,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let hook_first = TestHook {
        name: "hook_first",
        priority: ShutdownPriority::First,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let hook_normal = TestHook {
        name: "hook_normal",
        priority: ShutdownPriority::Normal,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };

    let mut registry = HookRegistry::new();
    registry.register(Box::new(hook_last));
    registry.register(Box::new(hook_first));
    registry.register(Box::new(hook_normal));

    assert_eq!(registry.len(), 3);
    registry.execute_all().await;

    assert_eq!(counter.load(Ordering::SeqCst), 3);

    let seq = sequence.lock().await;
    assert_eq!(*seq, vec!["hook_first", "hook_normal", "hook_last"]);
}

#[tokio::test]
async fn test_worker_manager_lifecycle() {
    let mut mgr = WorkerManager::new();
    let group = mgr.group("background-jobs");

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    group.spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        c.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(mgr.active_tasks(), 1);
    mgr.shutdown_all(Duration::from_secs(2)).await;
    assert_eq!(mgr.active_tasks(), 0);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
