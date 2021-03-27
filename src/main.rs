mod action;
mod application;
mod entity;
mod middleware;
mod web;

use crate::{
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
use tide::{http::cookies::SameSite, security::CorsMiddleware, sessions::SessionMiddleware, utils::After};

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
    // let mut app = tide::new();
    let mut app = tide::with_state(state.clone());

    // Middlewares
    let graceful_shutdown = GracefulShutdownMiddleware::new();
    let inner_error = After(log_inner_error);
    let cors = CorsMiddleware::new();
    let session = {
        let store = RedisStore::new(&envs.redis_uri).await?;
        let middleware = SessionMiddleware::new(store, &secret_key)
            .with_session_ttl(Some(Duration::from_secs(7200)))
            .with_same_site_policy(SameSite::Lax);
        middleware
    };
    let form_validation = FormValidationMiddleware::new(state.cipher.clone());

    app.with(graceful_shutdown.clone());
    app.with(cors);
    app.with(inner_error);
    app.with(session);
    app.with(form_validation);

    // Routes -----------------------------------------------------------------
    // To enable HTTP method deformation,
    // we have to split route server and nest it at root.
    let mut routes = tide::with_state(state);

    // Root
    routes.at("/").get(web::endpoint::index);
    routes.at("/public").serve_dir(&envs.public_dir)?;
    routes.at("/media").serve_dir(&envs.media_dir)?;

    // Authentication
    routes.at("/signin").get(web::endpoint::auth::render_signin);
    routes.at("/signin").post(web::endpoint::auth::signin);
    routes.at("/signout").delete(web::endpoint::auth::signout);

    // Media
    routes.at("/m/").get(web::endpoint::media::list_media);
    routes
        .at("/m/:hash_id")
        .get(web::endpoint::media::media)
        .patch(web::endpoint::media::update)
        .delete(web::endpoint::media::delete);
    routes.at("/upload").post(web::endpoint::media::upload);
    // Routes -----------------------------------------------------------------

    app.at("/").nest(routes);
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
