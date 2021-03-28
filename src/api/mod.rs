//! Contains API manipulations and types.

pub(crate) mod endpoint;
pub(crate) mod schema;

use crate::api::schema::ErrorResponse;

use async_trait::async_trait;
use tide::{http::StatusCode, Middleware, Next, Request, Result as TideResult};

/// Authorizes API call.
pub struct ApiAuthorizationMiddleware {
    token: String,
}

impl ApiAuthorizationMiddleware {
    /// Constructs a new middleware.
    pub fn new(token: &str) -> ApiAuthorizationMiddleware {
        ApiAuthorizationMiddleware { token: token.into() }
    }
}

#[async_trait]
impl<State: 'static + Send + Sync + Clone> Middleware<State> for ApiAuthorizationMiddleware {
    async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> TideResult {
        match request.header("Authorization") {
            Some(header) => {
                let mut header_values = header.as_str().split_ascii_whitespace();
                let auth_type = header_values.next();
                let token = header_values.next();
                match (auth_type, token) {
                    (Some("Bearer"), Some(t)) if t == self.token => (),
                    _ => return Ok(ErrorResponse::build(StatusCode::Forbidden, "Authorization failed")?),
                }
            }
            None => return Ok(ErrorResponse::build(StatusCode::Forbidden, "Authorization needed")?),
        }

        let response = next.run(request).await;
        Ok(response)
    }
}
