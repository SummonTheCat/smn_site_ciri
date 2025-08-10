use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::{CONTENT_TYPE, HeaderValue}};
use std::{convert::Infallible, path::PathBuf};
use tokio::{fs::File, io::AsyncReadExt};

use crate::plugins::plugin_components::{ComponentHandler, respond_status};

/// A minimal component that just returns static HTML.
/// - `name`: route name (e.g. "underConstruction")
/// - `path`: absolute or relative path to the HTML file to return
pub struct SimpleTemplateComponent {
    name: &'static str,
    path: PathBuf,
}

impl SimpleTemplateComponent {
    pub fn new(name_static: &'static str, path: PathBuf) -> Self {
        Self { name: name_static, path }
    }
}

#[async_trait]
impl ComponentHandler for SimpleTemplateComponent {
    fn component_name(&self) -> &'static str {
        self.name
    }

    async fn component_parse(
        &self,
        // If the loader already found a template by component name, prefer it.
        template: Option<String>,
        _args: Vec<String>,
    ) -> Result<Response<Body>, Infallible> {
        // 1) Use the template provided by the loader if available
        if let Some(tpl) = template {
            let mut resp = Response::new(Body::from(tpl));
            resp.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            return Ok(resp);
        }

        // 2) Otherwise read the file we were registered with
        match File::open(&self.path).await {
            Ok(mut f) => {
                let mut buf = Vec::new();
                if let Err(_) = f.read_to_end(&mut buf).await {
                    let mut r = respond_status(StatusCode::INTERNAL_SERVER_ERROR, "Failed to read component file");
                    r.headers_mut().insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("text/plain; charset=utf-8"),
                    );
                    return Ok(r);
                }
                let mut resp = Response::new(Body::from(buf));
                resp.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                );
                Ok(resp)
            }
            Err(_) => {
                let mut r = respond_status(StatusCode::NOT_FOUND, "Component file not found");
                r.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                Ok(r)
            }
        }
    }
}
