use std::{
    collections::HashMap,
    io::{Cursor, Read},
    str,
};

use anyhow::Result;
use multipart::server::Multipart;

/// Represents data of multipart field.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MultipartData {
    /// Text data
    Text(String),

    /// Bytes data (maybe text)
    Bytes(Vec<u8>),

    /// File data
    File(String, Vec<u8>),
}

impl MultipartData {
    /// Converts into string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MultipartData::Text(t) => Some(&t),
            MultipartData::Bytes(b) => str::from_utf8(&b).ok(),
            _ => None,
        }
    }
}

/// Parses multipart request body.
pub fn parse_multipart(boundary: &str, body: &[u8]) -> Result<HashMap<String, MultipartData>> {
    let mut multipart = Multipart::with_body(Cursor::new(&body[..]), boundary);
    let mut result = HashMap::new();

    while let Some(mut mpf) = multipart.read_entry()? {
        let field_name = mpf.headers.name.to_string();
        let field_type = mpf.headers.content_type;

        let mut field_data = Vec::new();
        mpf.data.read_to_end(&mut field_data)?;

        if let Some(filename) = mpf.headers.filename {
            result.insert(field_name, MultipartData::File(filename, field_data));
        } else {
            match field_type {
                Some(mime) if mime.essence_str() == "text/plain" => {
                    let text = String::from_utf8(field_data)?;
                    result.insert(field_name, MultipartData::Text(text));
                }
                Some(_) => {
                    result.insert(field_name, MultipartData::Bytes(field_data));
                }
                None => {
                    let text = String::from_utf8(field_data)?;
                    result.insert(field_name, MultipartData::Text(text));
                }
            }
        }
    }

    Ok(result)
}
