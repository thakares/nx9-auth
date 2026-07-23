use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use nx9_auth::config::Config;
use nx9_auth::runtime::{
    Application, HookRegistry, Lifecycle, RuntimeState, ShutdownCoordinator, ShutdownHook,
    ShutdownPriority, WorkerManager,
};

struct TestHook {
    name: &'static str,
    priority: ShutdownPriority,
    should_fail: bool,
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
        if self.should_fail {
            anyhow::bail!("deliberate hook failure");
        }
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
        should_fail: false,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let hook_first = TestHook {
        name: "hook_first",
        priority: ShutdownPriority::First,
        should_fail: false,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let hook_normal = TestHook {
        name: "hook_normal",
        priority: ShutdownPriority::Normal,
        should_fail: false,
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
async fn test_same_priority_hook_registration_order() {
    let counter = Arc::new(AtomicUsize::new(0));
    let sequence = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let hook_n1 = TestHook {
        name: "normal_1",
        priority: ShutdownPriority::Normal,
        should_fail: false,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let hook_n2 = TestHook {
        name: "normal_2",
        priority: ShutdownPriority::Normal,
        should_fail: false,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let hook_n3 = TestHook {
        name: "normal_3",
        priority: ShutdownPriority::Normal,
        should_fail: false,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };

    let mut registry = HookRegistry::new();
    registry.register(Box::new(hook_n1));
    registry.register(Box::new(hook_n2));
    registry.register(Box::new(hook_n3));

    registry.execute_all().await;
    let seq = sequence.lock().await;
    assert_eq!(*seq, vec!["normal_1", "normal_2", "normal_3"]);
}

#[tokio::test]
async fn test_hook_failure_resilience() {
    let counter = Arc::new(AtomicUsize::new(0));
    let sequence = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let failing_hook = TestHook {
        name: "failing_hook",
        priority: ShutdownPriority::Normal,
        should_fail: true,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };
    let succeeding_hook = TestHook {
        name: "succeeding_hook",
        priority: ShutdownPriority::Normal,
        should_fail: false,
        counter: counter.clone(),
        sequence: sequence.clone(),
    };

    let mut registry = HookRegistry::new();
    registry.register(Box::new(failing_hook));
    registry.register(Box::new(succeeding_hook));

    registry.execute_all().await;

    assert_eq!(counter.load(Ordering::SeqCst), 2);
    let seq = sequence.lock().await;
    assert_eq!(*seq, vec!["failing_hook", "succeeding_hook"]);
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

#[tokio::test]
async fn test_worker_live_forced_escalation_abort() {
    let mut mgr = WorkerManager::new();
    let group = mgr.group("long-worker");

    let worker_started = Arc::new(AtomicBool::new(false));
    let started = worker_started.clone();

    group.spawn(async move {
        started.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(10)).await;
    });

    // Wait for worker to begin execution
    while !worker_started.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert_eq!(mgr.active_tasks(), 1);

    let coord = ShutdownCoordinator::new();
    let coord_clone = coord.clone();

    let start_time = Instant::now();

    let shutdown_handle = tokio::spawn(async move {
        let mut m = mgr;
        m.shutdown_all_with_coordinator(Duration::from_secs(10), Some(&coord_clone))
            .await;
        m
    });

    // Short delay to ensure shutdown_all is actively waiting
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Trigger live second-signal forced escalation
    coord.cancel_forced();

    let mgr_after = shutdown_handle.await.expect("shutdown task join");
    let elapsed = start_time.elapsed();

    assert_eq!(mgr_after.active_tasks(), 0);
    assert!(
        elapsed < Duration::from_millis(1000),
        "Forced shutdown took {:?}, expected < 1s",
        elapsed
    );
}

#[tokio::test]
async fn test_worker_global_deadline_budget_across_groups() {
    let mut mgr = WorkerManager::new();
    mgr.group("group-a").spawn(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    mgr.group("group-b").spawn(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    mgr.group("group-c").spawn(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
    });

    assert_eq!(mgr.active_tasks(), 3);

    let start = Instant::now();
    mgr.shutdown_all(Duration::from_millis(200)).await;
    let elapsed = start.elapsed();

    assert_eq!(mgr.active_tasks(), 0);
    assert!(
        elapsed < Duration::from_millis(800),
        "Worker budget timeout across 3 groups took {:?}, expected single global deadline (~200ms)",
        elapsed
    );
}

#[tokio::test]
async fn test_forced_http_draining_escalation() -> anyhow::Result<()> {
    use axum::routing::get;

    let router = axum::Router::new().route(
        "/slow",
        get(|| async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            "done"
        }),
    );

    let mut config = Config::default();
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 0;
    config.database.url = Some("sqlite::memory:".to_string());

    let mut app = Application::builder(config).build().await?;
    app.router = Some(router);

    let coord = app.shutdown_coordinator().clone();
    let state_ref = app.state.clone();
    let port_ref = app.bound_port.clone();

    let app_task = tokio::spawn(async move { app.start().await });

    // Wait for server task to bind and store bound_port
    while port_ref.load(Ordering::Acquire) == 0 {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let port = port_ref.load(Ordering::Acquire);

    // Send HTTP request to /slow in background task (will take 10s if not aborted)
    let req_task = tokio::spawn(async move {
        if let Ok(mut stream) = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await {
            use tokio::io::AsyncWriteExt;
            let _ = stream
                .write_all(b"GET /slow HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
                .await;
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
        }
    });

    // Short delay for request to arrive at server
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Trigger 1st signal (graceful shutdown)
    coord.cancel_graceful();

    // Allow Tokio task executor to process cancellation and transition to Draining
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify RuntimeState is Draining while request is in-flight
    assert_eq!(state_ref.load(), RuntimeState::Draining);

    // Trigger 2nd signal (forced escalation)
    let start = Instant::now();
    coord.cancel_forced();

    let res = app_task.await?;
    let elapsed = start.elapsed();

    assert!(res.is_ok());
    assert!(
        elapsed < Duration::from_millis(1000),
        "Forced HTTP shutdown took {:?}, expected < 1s",
        elapsed
    );

    req_task.abort();
    Ok(())
}
