//! HTTP “middleware” for checking the x-service-key header.

use hyper::{Body, Request, Response, StatusCode};
use crate::sys_auth::core;

/// If the request carries a valid key, returns `None`.  
/// Otherwise returns a 401 response.
pub async fn handler_auth(req: &Request<Body>) -> Option<Response<Body>> {
    let hdr = req
        .headers()
        .get("x-service-key")
        .and_then(|h| h.to_str().ok());

    if let Some(key) = hdr {
        if core::verify(key) {
            return None;
        }
    }
    // fail
    Some(
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("Content-Type", "text/plain")
            .body(Body::from("Unauthorized: invalid or missing x-service-key"))
            .unwrap(),
    )
}
