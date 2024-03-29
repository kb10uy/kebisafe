mod action;
mod api;
mod application;
mod entity;
mod middleware;
mod web;

use crate::{
    api::ApiAuthorizationMiddleware,
    application::{Arguments, Environments, State, SubCommand},
    middleware::{log_inner_error, GracefulShutdownMiddleware},
    web::{deform_http_method, session::RedisStore, CsrfProtectionMiddleware, FormPreparseMiddleware},
};

use std::time::Duration;

use anyhow::{format_err, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use async_ctrlc::CtrlC;
use async_std::prelude::*;
use clap::Parser;
use flexi_logger::Logger;
use log::debug;
use rand::prelude::*;
use tide::{
    http::cookies::SameSite,
    security::CorsMiddleware,
    sessions::SessionMiddleware,
    utils::{After, Before},
};

#[async_std::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    Logger::try_with_env()?.start()?;

    let envs: Environments = envy::from_env()?;
    let args = Arguments::parse();

    match args.subcommand {
        Some(SubCommand::Serve) => run_server(envs).await?,
        Some(SubCommand::GeneratePassword) => generate_password().await?,
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
        let middleware = SessionMiddleware::new(store, &secret_key)
            .with_session_ttl(Some(Duration::from_secs(86400 * 7)))
            .with_same_site_policy(SameSite::Lax);
        middleware
    });
    web_routes.with(CsrfProtectionMiddleware::new(state.cipher.clone()));

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
    app.with(FormPreparseMiddleware);
    app.with(Before(deform_http_method));

    // Routes
    app.at("/").nest(web_routes);
    app.at("/api").nest(api_routes);
    app.at("/public").serve_dir(&envs.public_dir)?;
    app.at("/media").serve_dir(&envs.media_dir)?;

    // Start server
    let app_future = async { app.listen(envs.listen_at).await };
    let shutdown_future = async {
        let signal = CtrlC::new().expect("Cannot create signal");
        signal.await;
        graceful_shutdown.terminate().await;
        Ok(())
    };
    app_future.race(shutdown_future).await?;

    Ok(())
}

async fn generate_password() -> Result<()> {
    debug!("Generating password hash");

    let mut rng = thread_rng();
    let raw_password = rpassword::prompt_password("Type your password: ")?;

    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut rng);
    let password_hash = argon2
        .hash_password(raw_password.as_bytes(), salt.as_ref())
        .map_err(|_| format_err!("Failed to generate password hash"))?;

    println!("Success! Your password hash is below:");
    println!("{}", password_hash);

    Ok(())
}
