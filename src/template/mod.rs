//! Defines template types.

use crate::session::{Common, Flash};

use yarte::Template;

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub common: Common,
    pub pictures: Vec<String>,
}

#[derive(Debug, Template)]
#[template(path = "signin.html.hbs")]
pub struct Signin {
    pub common: Common,
}
