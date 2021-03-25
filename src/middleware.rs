//! Contains tide middlewares.

use crate::action::session::verify_csrf_token;

use async_std::sync::{Arc, RwLock};
use std::{
    io::{prelude::*, Cursor},
    process::exit,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use aes_gcm_siv::Aes256GcmSiv;
use anyhow::{bail, format_err, Result};
use async_trait::async_trait;
use log::{error, info, warn};
use multipart::server::Multipart;
use serde::Deserialize;
use tide::{
    http::{mime, Method, Request as HttpRequest, StatusCode},
    Middleware, Next, Request, Response, Result as TideResult,
};

const ALLOWED_METHODS: &[Method] = &[Method::Get, Method::Head, Method::Options];

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

/// Records client error logs.
pub struct ClientErrorLogMiddleware;

#[async_trait]
impl<State: 'static + Send + Sync + Clone> Middleware<State> for ClientErrorLogMiddleware {
    async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> TideResult {
        let response = next.run(request).await;
        if let Some(err) = response.error() {
            error!("Error response: {}", err);
        }

        Ok(response)
    }
}

/// Performs some actions for form data:
/// * Validate CSRF token of `_token`
/// * Deform HTTP method with `_method`
pub struct FormValidationMiddleware {
    cipher: Aes256GcmSiv,
}

impl FormValidationMiddleware {
    pub fn new(cipher: Aes256GcmSiv) -> FormValidationMiddleware {
        FormValidationMiddleware { cipher }
    }
}

#[async_trait]
impl<State: 'static + Send + Sync + Clone> Middleware<State> for FormValidationMiddleware {
    async fn handle(&self, mut request: Request<State>, next: Next<'_, State>) -> TideResult {
        #[derive(Debug, Default, Deserialize)]
        struct FormData {
            #[serde(rename = "_token")]
            token: Option<String>,

            #[serde(rename = "_method")]
            method: Option<String>,
        }

        if ALLOWED_METHODS.contains(&request.method()) {
            let response = next.run(request).await;
            return Ok(response);
        }

        info!("Attempting CSRF protection");
        let body_bytes = request.body_bytes().await?;
        let form_data = match request.content_type() {
            Some(m) if m.essence() == mime::FORM.essence() => serde_urlencoded::from_bytes(&body_bytes)?,
            Some(m) if m.essence() == mime::MULTIPART_FORM.essence() => {
                let mut multipart = Multipart::with_body(
                    Cursor::new(&body_bytes[..]),
                    m.param("boundary")
                        .ok_or_else(|| format_err!("Invalid multipart request"))?
                        .as_str(),
                );

                let mut form_data = FormData::default();
                while let Some(mut mpf) = multipart.read_entry()? {
                    match &mpf.headers.name[..] {
                        "_token" => {
                            let mut field_string = String::with_capacity(512);
                            mpf.data.read_to_string(&mut field_string)?;
                            form_data.token = Some(field_string);
                        }
                        "_method" => {
                            let mut field_string = String::with_capacity(512);
                            mpf.data.read_to_string(&mut field_string)?;
                            form_data.method = Some(field_string);
                        }
                        _ => continue,
                    }
                }

                form_data
            }
            m => {
                warn!("Unsupported Content-Type: {:?}", m);
                return Ok(Response::builder(StatusCode::BadRequest).build());
            }
        };

        // HTTP method deformation
        if let Some(method_str) = form_data.method {
            let method = method_str.parse()?;
            let inner_request: &mut HttpRequest = request.as_mut();
            inner_request.set_method(method);
            info!("Request method set to {}", method);
        }

        // CSRF token validation
        if !ALLOWED_METHODS.contains(&request.method()) {
            match form_data.token {
                Some(token) => match verify_csrf_token(&self.cipher, request.session(), &token) {
                    Ok(()) => info!("CSRF protection succeeded"),
                    Err(e) => {
                        warn!("CSRF protection failed: {}", e);
                        return Ok(Response::builder(StatusCode::BadRequest).build());
                    }
                },
                None => {
                    warn!("CSRF token missing");
                    return Ok(Response::builder(StatusCode::BadRequest).build());
                }
            }
        }

        request.set_body(body_bytes);
        let response = next.run(request).await;
        Ok(response)
    }
}
