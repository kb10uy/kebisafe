use crate::session::{Account, Flash};

use yarte::Template;

#[derive(Debug, Clone, Default)]
pub struct Common {
    pub account: Option<Account>,
    pub flashes: Vec<Flash>,
}

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub common: Common,
    pub pictures: Vec<String>,
}
