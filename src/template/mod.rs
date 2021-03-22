use yarte::Template;

#[derive(Debug, Clone, Default)]
pub struct Account {
    name: String,
}

#[derive(Debug, Clone)]
pub enum Flash {
    Info(String),
    Warning(String),
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct Common {
    account: Option<Account>,
    flashes: Vec<Flash>,
}

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub common: Common,
    pub pictures: Vec<String>,
}
