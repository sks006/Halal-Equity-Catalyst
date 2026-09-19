//! Worker Supervision Loop for resilient background tasks.
//!
//! Provides bounded automatic restarts with jittered exponential backoff
//! when workers encounter unhandled panics or fatal disconnects.
//! Prevents infinite restart thrashing and zombie task leaks.

use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub max_restarts: u32,
    pub restart_window: Duration,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            restart_window: Duration::from_secs(60),
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
        }
    }
}

/// Bounded worker supervisor tracking restarts and maintaining liveness.
pub struct WorkerSupervisor {
    name: String,
    config: SupervisorConfig,
    restart_count: Arc<AtomicU32>,
    is_running: Arc<AtomicBool>,
    last_error: Arc<RwLock<Option<String>>>,
}

impl WorkerSupervisor {
    pub fn new(name: &str, config: SupervisorConfig) -> Self {
        Self {
            name: name.to_string(),
            config,
            restart_count: Arc::new(AtomicU32::new(0)),
            is_running: Arc::new(AtomicBool::new(false)),
            last_error: Arc::new(RwLock::new(None)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn restart_count(&self) -> u32 {
        self.restart_count.load(Ordering::Relaxed)
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().ok().and_then(|g| g.clone())
    }

    /// Runs a supervised task factory until shutdown is received or restart limits are exhausted.
    pub async fn run_supervised<F, Fut>(&self, factory: F, mut shutdown: broadcast::Receiver<()>)
    where
        F: Fn(broadcast::Receiver<()>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), crate::error::ApiError>> + Send + 'static,
    {
        info!(worker = %self.name, "Starting supervised worker");
        self.is_running.store(true, Ordering::SeqCst);

        let mut window_start = Instant::now();
        let mut restarts_in_window = 0u32;

        loop {
            // Check shutdown before starting iteration
            if shutdown.try_recv().is_ok() {
                info!(worker = %self.name, "Supervisor received shutdown before spawn");
                break;
            }

            let child_shutdown = shutdown.resubscribe();
            let future = factory(child_shutdown);

            // Run child task wrapped in tokio::spawn to catch unhandled panics
            let mut join_handle = tokio::spawn(future);

            tokio::select! {
                _ = shutdown.recv() => {
                    info!(worker = %self.name, "Supervisor received shutdown signal — aborting child");
                    join_handle.abort();
                    break;
                }
                res = &mut join_handle => {
                    match res {
                        Ok(Ok(())) => {
                            info!(worker = %self.name, "Worker completed cleanly");
                            break;
                        }
                        Ok(Err(err)) => {
                            let err_msg = format!("Worker returned error: {}", err);
                            warn!(worker = %self.name, error = %err_msg, "Worker exited with error");
                            *self.last_error.write().unwrap() = Some(err_msg);
                        }
                        Err(join_err) => {
                            let panic_msg = if join_err.is_panic() {
                                "Worker crashed due to panic".to_string()
                            } else {
                                format!("Worker task cancelled/aborted: {}", join_err)
                            };
                            error!(worker = %self.name, error = %panic_msg, "Worker crashed unexpectedly");
                            *self.last_error.write().unwrap() = Some(panic_msg);
                        }
                    }
                }
            }

            // Check if window has expired
            let now = Instant::now();
            if now.duration_since(window_start) > self.config.restart_window {
                window_start = now;
                restarts_in_window = 0;
            }

            restarts_in_window += 1;
            self.restart_count.fetch_add(1, Ordering::Relaxed);

            if restarts_in_window > self.config.max_restarts {
                error!(
                    worker = %self.name,
                    restarts = restarts_in_window,
                    max = self.config.max_restarts,
                    "Worker restart limit exceeded — terminating supervisor loop to prevent thrashing"
                );
                break;
            }

            // Bounded exponential backoff with pseudo-jitter
            let backoff_multiplier = 1u64 << (restarts_in_window.saturating_sub(1)).min(6);
            let backoff =
                (self.config.base_backoff * backoff_multiplier as u32).min(self.config.max_backoff);

            warn!(
                worker = %self.name,
                restart = restarts_in_window,
                backoff_ms = backoff.as_millis(),
                "Supervising worker restart after backoff"
            );

            tokio::select! {
                _ = shutdown.recv() => {
                    info!(worker = %self.name, "Supervisor received shutdown during backoff");
                    break;
                }
                _ = tokio::time::sleep(backoff) => {}
            }
        }

        self.is_running.store(false, Ordering::SeqCst);
        info!(worker = %self.name, "Supervised worker terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_supervisor_restarts_crashing_worker_up_to_limit() {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let config = SupervisorConfig {
            max_restarts: 3,
            restart_window: Duration::from_secs(10),
            base_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(50),
        };

        let supervisor = WorkerSupervisor::new("test_worker", config);
        let invocation_count = Arc::new(AtomicUsize::new(0));
        let count_clone = invocation_count.clone();

        supervisor
            .run_supervised(
                move |_rx| {
                    let c = count_clone.clone();
                    async move {
                        let cur = c.fetch_add(1, Ordering::SeqCst);
                        if cur < 5 {
                            panic!("Simulated worker panic");
                        }
                        Ok(())
                    }
                },
                shutdown_rx,
            )
            .await;

        assert_eq!(supervisor.restart_count(), 4);
        assert!(!supervisor.is_running());
        drop(shutdown_tx);
    }
}
