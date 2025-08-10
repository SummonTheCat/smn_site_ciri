use async_trait::async_trait;
use hyper::{
    Body, Response, StatusCode,
    header::{CONTENT_TYPE, HeaderValue},
};
use std::convert::Infallible;

use crate::plugins::plugin_components::{ComponentHandler, respond_status};

/// Header component
/// Args: [section_heading]
pub struct CompHeader;

#[async_trait]
impl ComponentHandler for CompHeader {
    fn component_name(&self) -> &'static str {
        "header"
    }

    async fn component_parse(
        &self,
        template: Option<String>,
        args: Vec<String>,
    ) -> Result<Response<Body>, Infallible> {
        // Desired heading text, default if missing
        let section_heading = args.get(0)
            .map(|s| s.as_str())
            .unwrap_or("Technical Art");

        let tpl = match template {
            Some(t) => t,
            None => {
                let mut r = respond_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Header template not found: expected components/header.html (or components/header/template.html)"
                );
                r.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                return Ok(r);
            }
        };

        // Replace all {{section_heading}} placeholders
        let html = tpl.replace("{{section_heading}}", &html_escape(section_heading));

        let mut resp = Response::new(Body::from(html));
        resp.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        Ok(resp)
    }
}

/// Simple HTML escaper
fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            _ => c.to_string(),
        })
        .collect()
}
