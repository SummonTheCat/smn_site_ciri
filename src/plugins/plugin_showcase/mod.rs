use async_trait::async_trait;
use hyper::{
    Body, Request, Response, StatusCode,
    header::HeaderValue,
};
use smn_web_core::structs::struct_plugin::Plugin;
use std::{convert::Infallible};

mod html_builder;
mod html_markdown;
#[allow(unused)]
mod manager_list;
#[allow(unused)]
mod manager_project;

// ---------------------- Plugin ----------------------

pub struct PluginShowcase {}

impl PluginShowcase {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Plugin for PluginShowcase {
    async fn plugin_init(&mut self) {
        println!("{}", self.plugin_name());
    }

    fn plugin_name(&self) -> &str {
        "PluginShowcase"
    }

    fn plugin_can_handle(&self, req: &Request<Body>) -> bool {
        req.uri().path().starts_with("/projects")
    }

    async fn plugin_handle(
        &self,
        req: Request<Body>,
        _ctx: &smn_web_core::structs::struct_plugin::PluginContext,
    ) -> Result<Response<Body>, Infallible> {
        use hyper::header::{CONTENT_TYPE, LOCATION};

        let path = req.uri().path().to_string(); // e.g. "/projects/game_design/alchemists_convoy"
        let rel_full = strip_projects_prefix(&path).trim_matches('/'); // "game_design/alchemists_convoy" or ""

        // Get the project structure
        if rel_full.is_empty() {
            let project_structure =
                match manager_list::get_project_structure("data/displayProjectList.json") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to load project structure: {e}");
                        return Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("Internal Server Error"))
                            .unwrap());
                    }
                };

            let html =
                html_builder::generate_project_list_html(&project_structure, &path, rel_full);
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                )
                .body(Body::from(html))
                .unwrap());
        }

        // 2) Load structure to identify the project first.
        let project_structure =
            match manager_list::get_project_structure("data/displayProjectList.json") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to load project structure: {e}");
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Internal Server Error"))
                        .unwrap());
                }
            };

        // Find the deepest project whose path prefixes req path.
        let Some(project_node) = find_longest_matching_project(&project_structure, &path) else {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Project Not Found"))
                .unwrap());
        };

        // 3) Split into project path & remainder after project path.
        let project_abs_path = &project_node.path; // e.g. "/projects/game_design/alchemists_convoy"
        let remainder = path.strip_prefix(project_abs_path).unwrap_or("");
        let remainder = remainder.trim_start_matches('/');

        if !remainder.is_empty() {
            // We no longer serve resources here; base static server will handle any asset routes.
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap());
        }

        // Force trailing slash for nice relative behavior (optional)
        if !path.ends_with('/') {
            let location = format!("{}/", project_abs_path.trim_end_matches('/'));
            return Ok(Response::builder()
                .status(StatusCode::PERMANENT_REDIRECT) // 308 keeps method
                .header(LOCATION, location)
                .body(Body::empty())
                .unwrap());
        }

        // 4) Exact project hit → render project page (sidebar + content)
        let project_rel = strip_projects_prefix(project_abs_path).trim_start_matches('/'); // e.g. "game_design/alchemists_convoy"
        match manager_project::get_project_info("data/projectData", project_rel) {
            Ok(info) => {
                // Markdown path is now NEXT TO the projectData.json (not inside "resources")
                let md_text = match manager_project::load_markdown_content(
                    "data/projectData",
                    project_rel,
                    &info.project_content,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        if !info.project_content.trim().is_empty() {
                            eprintln!(
                                "Markdown load error: {e}. At path: {}",
                                info.project_content
                            );
                        }
                        String::new()
                    }
                };

                let html = html_builder::generate_project_page_html(
                    &project_structure,
                    &path,
                    rel_full,
                    &info,
                    &md_text,
                );
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(
                        CONTENT_TYPE,
                        HeaderValue::from_static("text/html; charset=utf-8"),
                    )
                    .body(Body::from(html))
                    .unwrap());
            }
            Err(e) => {
                eprintln!("Project info not found for '{}': {e}", project_rel);
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Project Not Found"))
                    .unwrap());
            }
        }
    }
}

// ========== UTILITIES  ==========

fn find_longest_matching_project<'a>(
    structure: &'a manager_list::ProjectStructure,
    req_path: &str,
) -> Option<&'a manager_list::Node> {
    let mut best: Option<&manager_list::Node> = None;
    let mut best_len = 0usize;

    for node in structure.iter() {
        let p = node.path.as_str();
        if req_path == p
            || (req_path.len() > p.len()
                && req_path.starts_with(p)
                && req_path.as_bytes().get(p.len()) == Some(&b'/'))
        {
            if p.len() > best_len {
                best_len = p.len();
                best = Some(node);
            }
        }
    }
    best
}

fn strip_projects_prefix(p: &str) -> &str {
    p.strip_prefix("/projects").unwrap_or(p)
}
