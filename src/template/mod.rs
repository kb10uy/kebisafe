use crate::session::{Common, Flash};

use yarte::Template;

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub common: Common,
    pub pictures: Vec<String>,
}
