//! HTTP glue: turn core results into hyper::Response<Body>.

use hyper::{Body, Request, Response, StatusCode, header::CONTENT_TYPE};
use multer::Multipart;
use serde_json;
use crate::sys_fileapi::core;
use tokio_util::io::ReaderStream;

pub async fn handler_upload(
    req: Request<Body>,
    base_url: &str,
) -> Response<Body> {
    // parse boundary
    let ct = match req.headers().get(CONTENT_TYPE).and_then(|h| h.to_str().ok()) {
        Some(ct) => ct,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Missing Content-Type"))
                .unwrap();
        }
    };
    let boundary = match multer::parse_boundary(ct) {
        Ok(b) => b,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Bad boundary: {}", e)))
                .unwrap();
        }
    };

    let mut multipart = Multipart::new(req.into_body(), boundary);
    let mut results = Vec::new();
    while let Some(field) = multipart.next_field().await.transpose() {
        match field {
            Ok(f) => {
                match core::api_upload_field(f, base_url).await {
                    Ok(info) => results.push(info),
                    Err(e) => {
                        eprintln!("upload error: {}", e);
                        return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("Upload failed"))
                            .unwrap();
                    }
                }
            }
            Err(e) => {
                eprintln!("multipart error: {}", e);
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Invalid form data"))
                    .unwrap();
            }
        }
    }

    let body = match serde_json::to_string(&results) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("json error: {}", e);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("JSON serialization error"))
                .unwrap();
        }
    };

    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

pub async fn handler_list() -> Response<Body> {
    match core::api_list_files().await {
        Ok(list) => {
            let body = serde_json::to_string(&list).unwrap_or("[]".into());
            Response::builder()
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap()
        }
        Err(e) => {
            eprintln!("list error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Could not list files"))
                .unwrap()
        }
    }
}

pub async fn handler_remove(filename: &str) -> Response<Body> {
    match core::api_remove_file(filename).await {
        Ok(true) => Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("Removed"))
            .unwrap(),
        Ok(false) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
        Err(e) => {
            eprintln!("remove error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Delete failed"))
                .unwrap()
        }
    }
}

/// The existing download handler can stay here:
pub async fn handler_download(filename: &str) -> Response<Body> {
    let path = std::path::PathBuf::from("uploads").join(filename);
    match tokio::fs::File::open(&path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            Response::builder()
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(Body::wrap_stream(stream))
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}
