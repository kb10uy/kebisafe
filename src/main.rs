mod application;
mod endpoint;
mod template;

use crate::application::{Arguments, Environments, State, Subcommand};

use anyhow::{format_err, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use clap::Clap;
use rand::prelude::*;

#[async_std::main]
async fn main() -> Result<()> {
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
    let mut app = tide::with_state(State::new("./dist")?);
    app.at("/public/*path").get(endpoint::public_static);
    app.at("/").get(endpoint::index);

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
