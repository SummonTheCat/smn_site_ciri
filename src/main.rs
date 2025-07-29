use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    http::header::CONTENT_TYPE,
    response::IntoResponse,
    routing::get,
    Router,
};
use once_cell::sync::Lazy;
use std::{fs, net::SocketAddr};
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

// ─── Cache index.html (with graceful fallback) ─────────────────────────────────
static INDEX_HTML: Lazy<Bytes> = Lazy::new(|| {
    match fs::read("res/index.html") {
        Ok(data) => Bytes::from(data),
        Err(err) => {
            // log the real I/O error to stderr
            eprintln!("Error loading res/index.html: {}", err);
            // return a minimal 500 page
            Bytes::from(
                "<!DOCTYPE html>\
                 <html><head><title>500</title></head>\
                 <body><h1>500 Internal Server Error</h1>\
                 <p>Unable to load index page.</p></body></html>"
            )
        }
    }
});

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/helloworld", get(hello_world))
        .fallback_service(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .service(ServeDir::new("static")),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// GET /
async fn index() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/html".parse().unwrap());

    // Always returns something—even if we fell back to the 500 page.
    (headers, INDEX_HTML.clone())
}

// GET /helloworld
async fn hello_world() -> impl IntoResponse {
    (StatusCode::OK, "Hello, world!\n")
}
