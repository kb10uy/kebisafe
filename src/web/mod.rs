//! Contains Web manipulations and endpoints.

pub(crate) mod endpoint;
pub(crate) mod multipart;
pub(crate) mod session;
pub(crate) mod template;

use crate::{
    action::session::verify_csrf_token,
    web::multipart::{parse_multipart, MultipartData},
};

use std::collections::HashMap;

use aes_gcm_siv::Aes256GcmSiv;
use anyhow::format_err;
use async_trait::async_trait;
use log::{info, warn};
use serde_json::Value as JsonValue;

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

struct ParsedForm(JsonValue);
struct ParsedMultipart(HashMap<String, MultipartData>);

/// Extends Request to cache form requests.
#[async_trait]
pub trait RequestPreParseExt {
    fn set_form(&mut self, form: JsonValue);
    fn set_multipart(&mut self, multipart: HashMap<String, MultipartData>);
    fn form_cache(&self) -> Option<&JsonValue>;
    fn multipart_cache(&self) -> Option<&HashMap<String, MultipartData>>;

    fn body_parsed_form(&self) -> &JsonValue {
        self.form_cache().expect("Form cache must be set")
    }

    fn body_parsed_multipart(&self) -> &HashMap<String, MultipartData> {
        self.multipart_cache().expect("Multipart cache must be set")
    }
}

impl<State: 'static + Send + Sync + Clone> RequestPreParseExt for Request<State> {
    fn set_form(&mut self, form: JsonValue) {
        self.set_ext(ParsedForm(form));
    }

    fn set_multipart(&mut self, multipart: HashMap<String, MultipartData>) {
        self.set_ext(ParsedMultipart(multipart));
    }

    fn form_cache(&self) -> Option<&JsonValue> {
        self.ext::<ParsedForm>().map(|m| &m.0)
    }

    fn multipart_cache(&self) -> Option<&HashMap<String, MultipartData>> {
        self.ext::<ParsedMultipart>().map(|m| &m.0)
    }
}

pub struct FormPreparseMiddleware;

#[async_trait]
impl<State: 'static + Send + Sync + Clone> Middleware<State> for FormPreparseMiddleware {
    async fn handle(&self, mut request: Request<State>, next: Next<'_, State>) -> TideResult {
        let content_type = match request.content_type() {
            Some(mime) if mime.essence() == mime::FORM.essence() => mime,
            Some(mime) if mime.essence() == mime::MULTIPART_FORM.essence() => mime,
            _ => return Ok(next.run(request).await),
        };

        let body_bytes = request.body_bytes().await?;
        if content_type.essence() == mime::FORM.essence() {
            let form_data: JsonValue = serde_json::from_slice(&body_bytes)?;
            request.set_form(form_data);
        } else {
            let boundary = content_type
                .param("boundary")
                .ok_or_else(|| format_err!("Invalid multipart request"))?
                .as_str();
            let multipart_data = parse_multipart(boundary, &body_bytes)?;
            request.set_multipart(multipart_data);
        }

        request.set_body(body_bytes);
        Ok(next.run(request).await)
    }
}

/// Deforms HTTP method based on its _method value.
pub async fn deform_http_method<State: 'static + Send + Sync + Clone>(mut request: Request<State>) -> Request<State> {
    if request.method() != Method::Post {
        return request;
    }

    let method = match request.content_type() {
        Some(mime) if mime.essence() == mime::FORM.essence() => {
            let value = request.body_parsed_form();
            value["_method"].as_str().and_then(|m| m.parse().ok())
        }
        Some(mime) if mime.essence() == mime::MULTIPART_FORM.essence() => {
            let value = request.body_parsed_multipart();
            value.get("_method").and_then(|v| v.as_str()).and_then(|m| m.parse().ok())
        }
        _ => return request,
    };

    if let Some(method) = method {
        let http_request: &mut HttpRequest = request.as_mut();
        http_request.set_method(method);
        info!("HTTP method set to {}", method)
    }

    request
}

/// Performs some actions for form data:
/// * Validate CSRF token of `_token`
/// * Deform HTTP method with `_method`
pub struct CsrfProtectionMiddleware {
    cipher: Aes256GcmSiv,
}

impl CsrfProtectionMiddleware {
    pub fn new(cipher: Aes256GcmSiv) -> CsrfProtectionMiddleware {
        CsrfProtectionMiddleware { cipher }
    }
}

#[async_trait]
impl<State: 'static + Send + Sync + Clone> Middleware<State> for CsrfProtectionMiddleware {
    async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> TideResult {
        if ALLOWED_METHODS.contains(&request.method()) {
            let response = next.run(request).await;
            return Ok(response);
        }

        info!("Attempting CSRF protection");
        let token = match request.content_type() {
            Some(mime) if mime.essence() == mime::FORM.essence() => {
                let value = request.body_parsed_form();
                value["_method"].as_str()
            }
            Some(mime) if mime.essence() == mime::MULTIPART_FORM.essence() => {
                let value = request.body_parsed_multipart();
                value.get("_method").and_then(|v| v.as_str())
            }
            _ => return Ok(next.run(request).await),
        };

        // CSRF token validation
        if !ALLOWED_METHODS.contains(&request.method()) {
            match token {
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

        Ok(next.run(request).await)
    }
}
