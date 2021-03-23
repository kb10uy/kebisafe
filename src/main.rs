mod application;
mod endpoint;
mod middleware;
mod session;
mod template;

use crate::{
    application::{Arguments, Environments, State, Subcommand},
    middleware::{ClientErrorLogMiddleware, FormValidationMiddleware},
};

use anyhow::{format_err, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use clap::Clap;
use rand::prelude::*;
use tide::{
    security::CorsMiddleware,
    sessions::{MemoryStore, SessionMiddleware},
};

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
    let (state, secret_key) = State::new(&envs, "./dist")?;
    let mut app = tide::with_state(state.clone());

    // Middlewares
    app.with(ClientErrorLogMiddleware);
    app.with(SessionMiddleware::new(MemoryStore::new(), &secret_key));
    app.with(CorsMiddleware::new());
    app.with(FormValidationMiddleware::new(state.cipher.clone()));

    // Routes -----------------------------------------------------------------
    // Root
    app.at("/").get(endpoint::index);
    app.at("/public/*path").get(endpoint::public_static);

    // Authentication
    app.at("/signin").post(endpoint::auth::signin);
    app.at("/signout").delete(endpoint::auth::signout);

    // Media
    app.at("/add").get(endpoint::add_flash);
    // Routes -----------------------------------------------------------------

    app.listen(envs.listen_at).await?;
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
