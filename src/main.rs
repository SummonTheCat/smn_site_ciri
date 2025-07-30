use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use std::{convert::Infallible, net::SocketAddr};

mod sys_fileapi {
    pub mod core;
    pub mod handlers;
}
mod sys_statichost {
    pub mod core;
    pub mod handlers;
}
mod sys_auth {
    pub mod core;
    pub mod handlers;
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let base_url = format!("http://{}", addr);

    // ensure dirs exist
    tokio::fs::create_dir_all("static").await.unwrap();
    tokio::fs::create_dir_all("uploads").await.unwrap();

    let make_svc = make_service_fn(move |_| {
        let base_url = base_url.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| router(req, base_url.clone())))
        }
    });

    println!("Listening on http://{}", addr);
    Server::bind(&addr).serve(make_svc).await.unwrap();
}

async fn router(req: Request<Body>, base_url: String) -> Result<Response<Body>, Infallible> {
    let method = req.method().clone();
    let uri = req.uri().path();

    // 0) Protect only upload and delete
    let is_protected = 
        (uri == "/api/upload" && method == Method::POST) ||
        (method == Method::DELETE && uri.starts_with("/api/files/"));

    if is_protected {
        if let Some(unauth) = sys_auth::handlers::handler_auth(&req).await {
            return Ok(unauth);
        }
    }

    // 1) API endpoints
    if uri == "/api/upload" && method == Method::POST {
        return Ok(sys_fileapi::handlers::handler_upload(req, &base_url).await);
    }
    if uri == "/api/files" && method == Method::GET {
        // public: list files
        return Ok(sys_fileapi::handlers::handler_list().await);
    }
    if method == Method::DELETE && uri.starts_with("/api/files/") {
        let filename = &uri["/api/files/".len()..];
        return Ok(sys_fileapi::handlers::handler_remove(filename).await);
    }

    // 2) Static UI
    if let Some(resp) = sys_statichost::handlers::handler_static(uri).await {
        return Ok(resp);
    }

    // 3) File downloads (public)
    if method == Method::GET && uri.starts_with("/files/") {
        let filename = &uri["/files/".len()..];
        return Ok(sys_fileapi::handlers::handler_download(filename).await);
    }

    // 4) 404 fallback
    Ok(Response::builder()
        .status(404)
        .body(Body::from("Not found"))
        .unwrap())
}
