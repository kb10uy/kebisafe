# Kebisafe
Minimal, single-user, and fast image upload service

## Requirements

### Deploy target
* PostgreSQL (12 or later)
* Redis (5 or later)

### Build environments
* Rust 1.51 or later
* Node.js 15 or later

## Usage
1. `yarn && yarn build`
2. `cargo build`
3. `cp .env.example .env` and edit
4. `cargo run`
