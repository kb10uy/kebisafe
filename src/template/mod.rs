//! Defines template types.

use crate::{
    action::session::{Common, Flash},
    application::State,
    entity::Media as MediaEntity,
};

use anyhow::Result;
use url::Url;
use yarte::Template;

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub url: Url,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<Url>,
}

#[allow(dead_code)]
impl PageInfo {
    /// Creates a new PageInfo with path.
    pub fn new(state: &State, path: &str) -> Result<PageInfo> {
        Ok(PageInfo {
            url: state.hosted_at.join(path)?,
            title: None,
            description: None,
            thumbnail: None,
        })
    }

    /// Sets the title.
    pub fn with_title(mut self, title: &str) -> PageInfo {
        self.title = Some(title.to_string());
        self
    }

    /// Sets the title.
    pub fn with_description(mut self, desc: &str) -> PageInfo {
        self.description = Some(desc.to_string());
        self
    }

    /// Sets the title.
    pub fn with_thumbnail(mut self, thumbnail: &Url) -> PageInfo {
        self.thumbnail = Some(thumbnail.clone());
        self
    }

    /// Returns OGP page type.
    pub fn page_type(&self) -> &str {
        if self.url.path() == "/" {
            "website"
        } else {
            "article"
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub info: PageInfo,
    pub common: Common,
    pub pictures_count: usize,
}

#[derive(Debug, Template)]
#[template(path = "signin.html.hbs")]
pub struct Signin {
    pub info: PageInfo,
    pub common: Common,
}

#[derive(Debug, Template)]
#[template(path = "media-list.html.hbs")]
pub struct MediaList {
    pub info: PageInfo,
    pub common: Common,
    pub media_list: Vec<MediaEntity>,
}

#[derive(Debug, Template)]
#[template(path = "media.html.hbs")]
pub struct Media {
    pub info: PageInfo,
    pub common: Common,
    pub media: MediaEntity,
}
