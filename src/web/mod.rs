//! Contains Web manipulations and endpoints.

pub(crate) mod endpoint;
pub(crate) mod multipart;
pub(crate) mod session;
pub(crate) mod template;

use crate::{action::session::verify_csrf_token, web::multipart::parse_multipart};

use aes_gcm_siv::Aes256GcmSiv;
use anyhow::format_err;
use async_trait::async_trait;
use log::{info, warn};
use serde::Deserialize;

use tide::{
    http::{mime, Method, Request as HttpRequest, StatusCode},
    Middleware, Next, Request, Response, Result as TideResult,
};

const ALLOWED_METHODS: &[Method] = &[Method::Get, Method::Head, Method::Options];

/// Ensures current session is signed in by owner.
/// If not, redirect to sign in page.
#[macro_export]
macro_rules! ensure_login {
    ($req:expr) => {{
        use tide::Redirect;
        use $crate::action::session::{get_account, swap_flashes, Flash};

        let session = $req.session_mut();
        match get_account(session) {
            Some(account) => account,
            None => {
                let mut old_flash = swap_flashes(session, vec![])?;
                old_flash.push(Flash::Info(format!("Please sign in")));
                swap_flashes(session, old_flash)?;
                return Ok(Redirect::new("/signin").into());
            }
        }
    }};
}

/// Validates form data.
/// If failed, add a flash message and redirect.
#[macro_export]
macro_rules! validate_form {
    ($t:ty, $req:expr, $floc:expr) => {{
        use tide::Redirect;
        use $crate::action::session::{swap_flashes, Flash};

        let form_data: Result<$t, _> = $req.body_form().await;
        match form_data {
            Ok(data) => data,
            Err(e) => {
                let session = $req.session_mut();
                let mut old_flash = swap_flashes(session, vec![])?;
                old_flash.push(Flash::Error(format!("Validation error: {}", e)));
                swap_flashes(session, old_flash)?;
                return Ok(Redirect::new($floc).into());
            }
        }
    }};
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
                let boundary = m
                    .param("boundary")
                    .ok_or_else(|| format_err!("Invalid multipart request"))?
                    .as_str();
                let multipart_data = parse_multipart(boundary, &body_bytes)?;

                FormData {
                    token: multipart_data.get("_token").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    method: multipart_data.get("_method").and_then(|d| d.as_str()).map(|s| s.to_string()),
                }
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
        // let session = request.session().clone();
        let response = next.run(request).await;
        // warn!("{:?}", session);
        Ok(response)
    }
}
