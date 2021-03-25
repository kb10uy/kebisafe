//! Defines template types.

use crate::{
    action::session::{Common, Flash},
    entity::Media as MediaEntity,
};

use yarte::Template;

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub common: Common,
    pub pictures_count: usize,
}

#[derive(Debug, Template)]
#[template(path = "signin.html.hbs")]
pub struct Signin {
    pub common: Common,
}

#[derive(Debug, Template)]
#[template(path = "media.html.hbs")]
pub struct Media {
    pub common: Common,
    pub media: MediaEntity,
}
