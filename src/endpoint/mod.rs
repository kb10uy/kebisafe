pub(crate) mod auth;

use crate::{
    application::State,
    session::{generate_csrf_token, swap_flashes, Flash},
    template,
};

use async_std::{
    fs,
    path::{Component as PathComponent, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use percent_encoding::percent_decode;
use tide::{
    http::{headers, mime, StatusCode},
    Redirect, Request, Response, Result as TideResult,
};
use yarte::Template;

/// `GET /`
/// Index
pub async fn index(mut request: Request<Arc<State>>) -> TideResult {
    let state = request.state().clone();
    let session = request.session_mut();

    let common = template::Common::from_session(session, vec![], Some(generate_csrf_token(&state.cipher, session)?))?;
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(template::Index { common, pictures: vec![] }.call()?)
        .build())
}

pub async fn add_flash(mut request: Request<Arc<State>>) -> TideResult {
    let session = request.session_mut();

    let new_flashes = vec![Flash::Info("Sample generated".to_string())];
    swap_flashes(session, new_flashes)?;

    Ok(Redirect::new("/").into())
}

/// `GET /public/*`
/// Serves all public static files.
pub async fn public_static(request: Request<Arc<State>>) -> TideResult {
    let state = request.state();
    let path = request.param("path").expect("Remaining path must be given");

    let canonical_path = canonicalize_path(&state.public_root, path)?;
    let length = match fs::metadata(&canonical_path).await {
        Ok(m) if m.is_file() => format!("{}", m.len()),
        _ => return Ok(Response::builder(StatusCode::NotFound).build()),
    };

    let mime_type = mime_guess::from_path(&canonical_path);
    let body = fs::read(canonical_path).await?;
    Ok(Response::builder(StatusCode::Ok)
        .header(headers::CONTENT_LENGTH, length)
        .content_type(mime_type.first_or_octet_stream().as_ref())
        .body(body)
        .build())
}

/// Canonicalizes input relative path.
fn canonicalize_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();

    let mut relative = PathBuf::new();
    for p in path.components() {
        match p {
            PathComponent::Normal(f) => {
                let part = f.to_str().context("Invalid path name")?;
                let decoded_part = percent_decode(part.as_bytes());
                let utf8_str = decoded_part.decode_utf8_lossy();
                relative.push(&*utf8_str);
            }
            PathComponent::ParentDir => {
                relative.pop();
            }
            _ => (),
        }
    }

    Ok(root.as_ref().join(relative))
}

/*
async fn index_test(mut request: Request<()>) -> TideResult {
    let content_type = request.content_type().unwrap();
    if content_type.essence() != mime::MULTIPART_FORM.essence() {
        return Ok(Response::builder(StatusCode::BadRequest)
            .body("You must upload at least one file")
            .build());
    }
    let boundary = match content_type.param("boundary") {
        Some(b) => b.as_str(),
        None => return Ok(Response::builder(StatusCode::BadRequest).body("Invalid multipart request").build()),
    };

    let body = request.body_bytes().await?;
    let wrapped_body = Cursor::new(&body[..]);
    let mut multipart = Multipart::with_body(wrapped_body, boundary);

    loop {
        let part = match multipart.read_entry()? {
            Some(part) => part,
            None => break,
        };
        let MultipartField { headers, mut data } = part;

        match headers.filename {
            Some(f) => {
                let mut bytes = vec![];
                data.read_to_end(&mut bytes)?;
                info!("File {} given, size was {} bytes", f, bytes.len())
            }
            None => info!("Non-file given"),
        }
    }

    todo!();
}
*/
