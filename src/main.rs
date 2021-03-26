mod action;
mod application;
mod endpoint;
mod entity;
mod middleware;
mod template;

use crate::{
    application::{Arguments, Environments, RedisStore, State, Subcommand},
    middleware::{ClientErrorLogMiddleware, FormValidationMiddleware, GracefulShutdownMiddleware},
};

use anyhow::{format_err, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use clap::Clap;
use rand::prelude::*;
use tide::{security::CorsMiddleware, sessions::SessionMiddleware};

#[async_std::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    let envs: Environments = envy::from_env()?;
    let args = Arguments::parse();

    match args.subcommand {
        Some(Subcommand::Serve) => run_server(envs).await?,
        Some(Subcommand::GeneratePassword) => generate_password().await?,
        None => run_server(envs).await?,
    }

    Ok(())
}

async fn run_server(envs: Environments) -> Result<()> {
    let (state, secret_key) = State::new(&envs).await?;
    let mut app = tide::new();

    // Middlewares
    let graceful = GracefulShutdownMiddleware::new();
    app.with(graceful.clone());
    app.with(ClientErrorLogMiddleware);
    app.with(SessionMiddleware::new(RedisStore::new(&envs.redis_uri).await?, &secret_key));
    app.with(CorsMiddleware::new());
    app.with(FormValidationMiddleware::new(state.cipher.clone()));

    // Routes -----------------------------------------------------------------
    // To enable HTTP method deformation,
    // we have to split route server and nest it at root.
    let mut routes = tide::with_state(state);

    // Root
    routes.at("/").get(endpoint::index);
    routes.at("/public").serve_dir(&envs.public_dir)?;
    routes.at("/media").serve_dir(&envs.media_dir)?;

    // Authentication
    routes.at("/signin").get(endpoint::auth::render_signin);
    routes.at("/signin").post(endpoint::auth::signin);
    routes.at("/signout").delete(endpoint::auth::signout);

    // Media
    routes.at("/m/").get(endpoint::media::list_media);
    routes.at("/m/:hash_id").get(endpoint::media::media);
    routes.at("/upload").post(endpoint::media::upload);
    // Routes -----------------------------------------------------------------

    app.at("/").nest(routes);
    app.listen(envs.listen_at).await?;
    graceful.terminate().await;
    Ok(())
}

async fn generate_password() -> Result<()> {
    let mut rng = thread_rng();
    let raw_password = rpassword::read_password_from_tty(Some("Type your password: "))?;

    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut rng);
    let password_hash = argon2
        .hash_password_simple(raw_password.as_bytes(), salt.as_ref())
        .map_err(|_| format_err!("Failed to generate password hash"))?;

    println!("Success! Your password hash is below:");
    println!("{}", password_hash);

    Ok(())
}
