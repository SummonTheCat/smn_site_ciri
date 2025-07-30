//! HTTP glue: serve whatever `core::map_static_path` gives us.

use std::str::FromStr;

use hyper::{Body, Response, StatusCode};
use hyper::header::CONTENT_TYPE;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use mime_guess::{mime, Mime};

/// Try to serve a file for this URI under `static/`.
/// Returns `Some(response)` if `uri` is a static route, or `None` otherwise.
pub async fn handler_static(uri: &str) -> Option<Response<Body>> {
    if let Some(path) = crate::sys_statichost::core::map_static_path(uri) {
        match File::open(&path).await {
            Ok(file) => {
                let stream = ReaderStream::new(file);
                let mime = Mime::from_str(&mime_guess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string())
                    .unwrap_or(mime::TEXT_PLAIN);
                let resp = Response::builder()
                    .header(CONTENT_TYPE, mime.as_ref())
                    .body(Body::wrap_stream(stream))
                    .unwrap();
                Some(resp)
            }
            Err(_) => Some(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not found"))
                    .unwrap()
            ),
        }
    } else {
        None
    }
}
