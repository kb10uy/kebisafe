mod action;
mod api;
mod application;
mod entity;
mod middleware;
mod web;

use crate::{
    api::ApiAuthorizationMiddleware,
    application::{Arguments, Environments, State, Subcommand},
    middleware::{log_inner_error, GracefulShutdownMiddleware},
    web::{session::RedisStore, FormValidationMiddleware},
};

use std::time::Duration;

use anyhow::{format_err, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use clap::Clap;
use log::debug;
use rand::prelude::*;
use tide::{security::CorsMiddleware, sessions::SessionMiddleware, utils::After};

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
    debug!("Started to run server");

    let (state, secret_key) = State::new(&envs).await?;

    // Web Routes -------------------------------------------------------------
    // To enable HTTP method deformation,
    // we have to split route server and nest it at root.
    let mut web_routes = tide::with_state(state.clone());
    web_routes.with({
        let store = RedisStore::new(&envs.redis_uri).await?;
        let middleware = SessionMiddleware::new(store, &secret_key).with_session_ttl(Some(Duration::from_secs(7200)));
        middleware
    });
    web_routes.with(FormValidationMiddleware::new(state.cipher.clone()));

    // Root
    web_routes.at("/").get(web::endpoint::index);

    // Authentication
    web_routes
        .at("/signin")
        .get(web::endpoint::auth::render_signin)
        .post(web::endpoint::auth::signin);
    web_routes.at("/signout").delete(web::endpoint::auth::signout);

    // Media
    web_routes
        .at("/m/")
        .get(web::endpoint::media::list_media)
        .post(web::endpoint::media::upload);
    web_routes
        .at("/m/:hash_id")
        .get(web::endpoint::media::media)
        .patch(web::endpoint::media::update)
        .delete(web::endpoint::media::delete);

    // API Routes -------------------------------------------------------------
    let mut api_routes = tide::with_state(state.clone());
    api_routes.with(ApiAuthorizationMiddleware::new(&envs.api_token));

    api_routes.at("/show").get(api::endpoint::show);
    api_routes.at("/upload").post(api::endpoint::upload);

    // Root App --------------------------------------------------------------
    let mut app = tide::new();

    // Middlewares
    let graceful_shutdown = GracefulShutdownMiddleware::new();
    app.with(graceful_shutdown.clone());
    app.with(After(log_inner_error));
    app.with(CorsMiddleware::new());

    // Routes
    app.at("/").nest(web_routes);
    app.at("/api").nest(api_routes);
    app.at("/public").serve_dir(&envs.public_dir)?;
    app.at("/media").serve_dir(&envs.media_dir)?;

    // Start server
    app.listen(envs.listen_at).await?;
    graceful_shutdown.terminate().await;
    Ok(())
}

async fn generate_password() -> Result<()> {
    debug!("Generating password hash");

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
