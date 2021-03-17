mod application;
mod endpoint;
mod template;

use crate::application::{State, Environments};

use anyhow::Result;

#[async_std::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let envs: Environments = envy::from_env()?;

    let mut app = tide::with_state(State {});
    app.at("/").get(endpoint::index);

    app.listen(envs.listen_at).await?;
    Ok(())
}
