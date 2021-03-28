//! Contains tide middlewares.

use async_std::sync::{Arc, RwLock};
use std::{
    process::exit,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use anyhow::{bail, Result};
use async_trait::async_trait;
use log::{error, info};
use tide::{Middleware, Next, Request, Response, Result as TideResult};

/// Records client error logs.
pub async fn log_inner_error(response: Response) -> TideResult {
    if let Some(err) = response.error() {
        error!("Error response: {}", err);
    }
    Ok(response)
}

/// Performs graceful shutdown.
#[derive(Debug, Clone)]
struct GracefulShutdownBox {
    terminating: Arc<AtomicBool>,
    in_process: Arc<AtomicU32>,
}

impl Drop for GracefulShutdownBox {
    fn drop(&mut self) {
        let previous_count = self.in_process.fetch_sub(1, Ordering::Release);
        let should_exit = self.terminating.load(Ordering::Acquire);
        if previous_count == 1 && should_exit {
            exit(0);
        }
    }
}

/// Deals with graceful shutdown.
#[derive(Debug, Clone)]
pub struct GracefulShutdownMiddleware {
    shutdown_box: Arc<RwLock<Option<GracefulShutdownBox>>>,
}

impl GracefulShutdownMiddleware {
    /// Constructs a new middleware.
    pub fn new() -> GracefulShutdownMiddleware {
        let terminating = Arc::new(AtomicBool::new(false));
        let in_process = Arc::new(AtomicU32::new(1));
        let shutdown_box = Arc::new(RwLock::new(Some(GracefulShutdownBox { terminating, in_process })));

        GracefulShutdownMiddleware { shutdown_box }
    }

    /// Reserves a new Box if not terminating.
    async fn reserve(&self) -> Result<GracefulShutdownBox> {
        let locked = self.shutdown_box.read().await;
        match locked.as_ref() {
            Some(sb) => {
                let reserved_box = sb.clone();
                reserved_box.in_process.fetch_add(1, Ordering::Acquire);
                Ok(reserved_box)
            }
            None => bail!("Already started to terminate"),
        }
    }

    /// Starts to terminate.
    pub async fn terminate(&self) {
        let mut locked = self.shutdown_box.write().await;
        match locked.as_mut() {
            Some(sb) => {
                sb.terminating.store(true, Ordering::Release);
                let _ = locked.take();
                info!("Termination ordered");
            }
            None => (),
        }
    }
}

#[async_trait]
impl<State: 'static + Send + Sync + Clone> Middleware<State> for GracefulShutdownMiddleware {
    async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> TideResult {
        let _reserved = self.reserve().await?;
        let response = next.run(request).await;

        Ok(response)
    }
}
