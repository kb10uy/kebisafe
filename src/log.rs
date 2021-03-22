use async_trait::async_trait;
use log::error;
use tide::{Middleware, Next, Request, Result as TideResult};

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
