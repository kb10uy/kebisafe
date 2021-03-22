use yarte::Template;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default)]
pub struct Account {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Flash {
    Info(String),
    Warning(String),
    Error(String),
}

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
