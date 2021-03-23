//! Contains tide middlewares.

use crate::session::verify_csrf_token;

use std::io::{prelude::*, Cursor};

use aes_gcm_siv::Aes256GcmSiv;
use anyhow::format_err;
use async_trait::async_trait;
use log::error;
use multipart::server::Multipart;
use serde::Deserialize;
use tide::{
    http::{mime, Method, Request as HttpRequest, StatusCode},
    Middleware, Next, Request, Response, Result as TideResult,
};

const ALLOWED_METHODS: &[Method] = &[Method::Get, Method::Head, Method::Options];

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

        let body_bytes = request.body_bytes().await?;
        let form_data = match request.content_type() {
            Some(m) if m == mime::FORM => serde_urlencoded::from_bytes(&body_bytes)?,
            Some(m) if m == mime::MULTIPART_FORM => {
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
            _ => return Ok(Response::builder(StatusCode::BadRequest).build()),
        };

        // HTTP method deformation
        if let Some(method_str) = form_data.method {
            let method = method_str.parse()?;
            let inner_request: &mut HttpRequest = request.as_mut();
            inner_request.set_method(method);
        }

        // CSRF token validation
        if !ALLOWED_METHODS.contains(&request.method()) {
            match form_data.token {
                Some(token) if verify_csrf_token(&self.cipher, request.session(), &token).is_ok() => (),
                _ => return Ok(Response::builder(StatusCode::BadRequest).build()),
            }
        }

        request.set_body(body_bytes);
        let response = next.run(request).await;
        Ok(response)
    }
}
