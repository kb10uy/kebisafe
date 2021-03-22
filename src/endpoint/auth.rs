use crate::{application::State, template};

use async_std::{
    path::{Component as PathComponent, Path, PathBuf},
};

use anyhow::{Context, Result};
use percent_encoding::percent_decode;
use tide::{
    http::{headers, mime, StatusCode},
    sessions::Session,
    Redirect, Request, Response, Result as TideResult,
};
use yarte::Template;

pub async fn signin(mut request: Request<State>) -> TideResult {
    let session = request.session_mut();
    todo!();
}
