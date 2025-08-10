use async_trait::async_trait;
use hyper::{
    body::to_bytes,
    header::{CONTENT_TYPE, HeaderValue},
    Body, Method, Request, Response, StatusCode,
};
use serde::Deserialize;
use smn_web_core::structs::struct_plugin::Plugin;
use std::{
    collections::HashMap,
    convert::Infallible,
    path::{Path, PathBuf},
};
use tokio::{fs::File, io::AsyncReadExt};

use crate::plugins::plugin_components::components::comp_simple::SimpleTemplateComponent;

pub mod components;

// ---------------------- Component system ----------------------

#[async_trait]
pub trait ComponentHandler: Send + Sync {
    /// Programmatic name for routing: e.g. "simple_button"
    fn component_name(&self) -> &'static str;

    /// Process the component request using the (optional) template contents and args.
    /// Return a full HTTP response (set Content-Type as appropriate).
    async fn component_parse(
        &self,
        template: Option<String>,
        args: Vec<String>,
    ) -> Result<Response<Body>, Infallible>;
}

// ---------------------- Plugin ----------------------

pub struct PluginComponents {
    handlers: HashMap<&'static str, Box<dyn ComponentHandler>>,
}

impl PluginComponents {
    /// Default constructor: empty registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler. Call this from `plugin_init`.
    pub fn register<H: ComponentHandler + 'static>(&mut self, handler: H) {
        self.handlers.insert(handler.component_name(), Box::new(handler));
    }
}

#[async_trait]
impl Plugin for PluginComponents {
    async fn plugin_init(&mut self) {
        println!(
            "{} initialized with {} handler(s)",
            self.plugin_name(),
            self.handlers.len()
        );
    }

    fn plugin_name(&self) -> &str {
        "PluginComponents"
    }

    fn plugin_can_handle(&self, req: &Request<Body>) -> bool {
        req.uri().path().starts_with("/components")
    }

    async fn plugin_handle(&self, req: Request<Body>, _ctx: &smn_web_core::structs::struct_plugin::PluginContext) -> Result<Response<Body>, Infallible> {
        let method = req.method().clone();
        let path = req.uri().path().to_string(); // Extract path as String to avoid borrow after move

        // Route base: "/components"
        // For static files, we map "/components/**" -> "./components/**" (without double-joining).
        // For component processing, we look at "/components/<name>" where <name> has no '.' and matches a registered handler.
        let after_prefix = path.trim_start_matches("/components/");
        let is_root = after_prefix.is_empty();

        // If the path looks like a concrete file (has '.' in last segment) or it's the directory root,
        // try static serving first.
        let looks_like_file = after_prefix
            .rsplit('/')
            .next()
            .map(|seg| seg.contains('.'))
            .unwrap_or(false);

        // If it's a registered component path (no '.'), and we have a handler -> process
        if !is_root && !looks_like_file {
            if let Some(seg) = after_prefix.split('/').next() {
                if let Some(handler) = self.handlers.get(seg) {
                    return process_component_request(handler, seg, &method, req).await;
                }
            }
        }

        // Fallback: static file hosting
        serve_static(after_prefix).await
    }
}

impl PluginComponents {
    /// Register a simple, no-logic component by pointing at an HTML file.
    /// The route name is derived from the file stem.
    /// Example: "./components/underConstruction.html" -> route "underConstruction"
    pub fn register_simple<P: AsRef<std::path::Path>>(&mut self, path: P) {
        let pb = path.as_ref().to_path_buf();

        // Derive route name from file stem
        let stem = pb.file_stem()
            .and_then(|s| s.to_str())
            .expect("register_simple: could not derive component name from path (missing file stem)");

        // Leak the name to get a &'static str (handlers live for program lifetime)
        let leaked: &'static str = Box::leak(stem.to_string().into_boxed_str());

        // Insert handler
        let handler = SimpleTemplateComponent::new(leaked, pb);
        self.handlers.insert(leaked, Box::new(handler));
    }
}


// ---------------------- Static serving ----------------------

async fn serve_static(safe_rel_path: &str) -> Result<Response<Body>, Infallible> {
    // Harden path traversal: reject any ".." segments
    if safe_rel_path.split('/').any(|s| s == "..") {
        return Ok(respond_status(
            StatusCode::FORBIDDEN,
            "403 Forbidden: invalid path",
        ));
    }

    // Map to "./components/<safe_rel_path>" (or directory index if empty)
    let base = PathBuf::from("./components");
    let target_path = if safe_rel_path.is_empty() {
        base.join("index.html")
    } else {
        base.join(safe_rel_path)
    };

    // If it's a directory, try index.html inside it
    let final_path = if is_dir(&target_path).await {
        target_path.join("index.html")
    } else {
        target_path
    };

    // Try to open and return
    if let Some((p, bytes)) = try_open(&final_path).await {
        return Ok(ok_with_type(bytes, guess_content_type(&p)));
    }

    Ok(respond_status(StatusCode::NOT_FOUND, "404 Not Found"))
}

async fn is_dir(path: &Path) -> bool {
    match tokio::fs::metadata(path).await {
        Ok(md) => md.is_dir(),
        Err(_) => false,
    }
}

async fn try_open(path: &Path) -> Option<(String, Vec<u8>)> {
    if tokio::fs::metadata(path).await.ok()?.is_file() {
        if let Ok(mut f) = File::open(path).await {
            let mut contents = Vec::new();
            if f.read_to_end(&mut contents).await.is_ok() {
                return Some((path.to_string_lossy().into_owned(), contents));
            }
        }
    }
    None
}

fn ok_with_type(body: Vec<u8>, content_type: &'static str) -> Response<Body> {
    let mut resp = Response::new(Body::from(body));
    resp.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    resp
}

fn respond_status(code: StatusCode, msg: &str) -> Response<Body> {
    let mut r = Response::new(Body::from(msg.to_string()));
    *r.status_mut() = code;
    r
}

fn guess_content_type(path: &str) -> &'static str {
    if let Some(ext) = Path::new(path).extension().and_then(|s| s.to_str()) {
        match ext {
            "html" | "htm" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "wasm" => "application/wasm",
            "txt" => "text/plain; charset=utf-8",
            _ => "application/octet-stream",
        }
    } else {
        "application/octet-stream"
    }
}

// ---------------------- Component request plumbing ----------------------

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ComponentPayload {
    #[serde(default)]
    compArgs: Vec<String>,
}

async fn process_component_request(
    handler: &Box<dyn ComponentHandler>,
    component_name: &str,
    method: &Method,
    mut req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Load optional template file: ./components/<component_name>/template.html
    // If not found, pass None to the handler.
    let template_path_a = PathBuf::from("./components")
        .join(component_name)
        .join("template.html");
    let template_path_b = PathBuf::from("./components").join(format!("{component_name}.html"));

    let template = if let Some((_p, bytes)) = try_open(&template_path_a).await {
        Some(String::from_utf8_lossy(&bytes).into_owned())
    } else if let Some((_p, bytes)) = try_open(&template_path_b).await {
        Some(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        None
    };

    // Extract args: prefer POST JSON body { "compArgs": ["..."] }
    let args = if *method == Method::POST {
        let full = to_bytes(req.body_mut()).await.unwrap_or_default();
        if full.is_empty() {
            Vec::new()
        } else {
            serde_json::from_slice::<ComponentPayload>(&full)
                .map(|p| p.compArgs)
                .unwrap_or_default()
        }
    } else {
        // Optional: allow GET ?compArgs=... (comma-separated) as a convenience
        let query = req.uri().query().unwrap_or_default();
        parse_args_from_query(query)
    };

    handler.component_parse(template, args).await
}

fn parse_args_from_query(qs: &str) -> Vec<String> {
    // Very small utility: compArgs=msg,url
    qs.split('&')
        .find_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or_default();
            if k == "compArgs" {
                Some(
                    urlencoding::decode(v)
                        .ok()
                        .map(|s| {
                            s.split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

