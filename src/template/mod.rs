use crate::session::{swap_flashes, Account, Flash};

use anyhow::Result;
use tide::sessions::Session;
use yarte::Template;

#[derive(Debug, Clone, Default)]
pub struct Common {
    pub account: Option<Account>,
    pub flashes: Vec<Flash>,
    pub csrf: String,
}

impl Common {
    pub fn from_session(session: &mut Session, new_flashes: Vec<Flash>, csrf: Option<String>) -> Result<Common> {
        Ok(Common {
            account: None,
            flashes: swap_flashes(session, new_flashes)?,
            csrf: csrf.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub common: Common,
    pub pictures: Vec<String>,
}
