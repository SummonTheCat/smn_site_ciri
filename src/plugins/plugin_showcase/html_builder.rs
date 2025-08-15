use std::fs;
use crate::plugins::plugin_showcase::{html_markdown, manager_list, manager_project};

// Where we load the page template from.
const TEMPLATE_PATH: &str = "data/templates/projectpage.html";

pub fn generate_project_list_html(
    project_structure: &manager_list::ProjectStructure,
    req_path: &str,
    path_relative: &str,
) -> String {
    let template = load_template();
    let sidebar = render_sidebar_html(project_structure, req_path, path_relative);
    let content = String::new();
    let title = "Projects";
    apply_template(&template, title, &sidebar, &content)
}

pub fn generate_project_page_html(
    project_structure: &manager_list::ProjectStructure,
    req_path: &str,
    path_relative: &str,
    info: &manager_project::ProjectInfo,
    md_text: &str,
) -> String {
    let template = load_template();
    let sidebar = render_sidebar_html(project_structure, req_path, path_relative);

    let mut content = String::new();

    // 1) Heading
    content.push_str(&format!(
        r#"<h1 class="project-title">{}</h1>"#,
        html_escape(&info.project_name)
    ));

    // 2) Description
    if !info.project_description.is_empty() {
        content.push_str(&format!(
            r#"<p class="project-description">{}</p>"#,
            html_escape(&info.project_description)
        ));
    }

    // 3) State (little box)
    if !info.project_state.is_empty() {
        content.push_str(r#"<div class="state-box project-state">"#);
        content.push_str(r#"<span class="state-dot"></span>"#);
        content.push_str(r#"<span class="label">State:</span> "#);
        content.push_str(&format!(
            r#"<span class="value">{}</span>"#,
            html_escape(&info.project_state)
        ));
        content.push_str("</div>");
    }

    // 4) Videos (AS-IS paths from JSON)
    if !info.project_videos.is_empty() {
        content.push_str(r#"<section class="project-videos"><div class="video-grid">"#);
        for vid in &info.project_videos {
            let src = html_escape(vid);
            content.push_str(&format!(
                r#"<video class="video-item" controls preload="metadata" src="{}"></video>"#,
                src
            ));
        }
        content.push_str("</div></section>");
    }

    // 5) Images (AS-IS paths from JSON)
    if !info.project_images.is_empty() {
        content.push_str(r#"<section class="project-images"><div class="image-grid">"#);
        for img in &info.project_images {
            let src = html_escape(img);
            content.push_str(&format!(
                r#"<img class="image-item" src="{}" alt="" loading="lazy"/>"#,
                src
            ));
        }
        content.push_str("</div></section>");
    }

    // 6) Content (Markdown → HTML)
    if !md_text.trim().is_empty() {
        let md_html = html_markdown::render_markdown(md_text);
        content.push_str(r#"<section class="project-content">"#);
        content.push_str(&md_html);
        content.push_str("</section>");
    }

    // 7 & 8) Tools and Links (two columns)
    if !info.project_tools.is_empty() || !info.project_links.is_empty() {
        content.push_str(r#"<section class="meta-grid">"#);

        // Tools card
        if !info.project_tools.is_empty() {
            content.push_str(r#"<div class="meta-card"><h2 class="section-title">Tools</h2>"#);
            content.push_str(r#"<table class="mini-table"><tbody>"#);
            for t in &info.project_tools {
                content.push_str(r#"<tr>"#);
                content.push_str(&format!(
                    r#"<td class="cell-value">{}</td>"#,
                    html_escape(t)
                ));
                content.push_str(r#"</tr>"#);
            }
            content.push_str(r#"</tbody></table></div>"#);
        }

        // Links card
        if !info.project_links.is_empty() {
            content.push_str(r#"<div class="meta-card"><h2 class="section-title">Links</h2>"#);
            content.push_str(r#"<table class="mini-table"><tbody>"#);
            for l in &info.project_links {
                let href = html_escape(&l.link);
                let desc = html_escape(&l.description);
                content.push_str(r#"<tr>"#);
                content.push_str(&format!(
                    r#"<td class="cell-value"><a class="link" href="{}" target="_blank" rel="noopener">{}</a></td>"#,
                    href, desc
                ));
                content.push_str(r#"</tr>"#);
            }
            content.push_str(r#"</tbody></table></div>"#);
        }

        content.push_str("</section>");
    }

    apply_template(&template, &info.project_name, &sidebar, &content)
}

// ------------- helpers -------------

fn load_template() -> String {
    match fs::read_to_string(TEMPLATE_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "WARN: Failed to read template at '{}': {}. Using built-in fallback.",
                TEMPLATE_PATH, e
            );
            fallback_template()
        }
    }
}

fn apply_template(template: &str, title: &str, sidebar_html: &str, content_html: &str) -> String {
    template
        .replace("{{TITLE}}", &html_escape(title))
        .replace("{{SIDEBAR}}", sidebar_html)
        .replace("{{CONTENT}}", content_html)
}

fn render_sidebar_html(
    project_structure: &manager_list::ProjectStructure,
    req_path: &str,
    path_relative: &str,
) -> String {
    let mut html = String::new();
    html.push_str(r#"<nav class="sidebar-nav">"#);
    html.push_str(r#"<ul class="project-list level-0">"#);
    for node in project_structure.roots() {
        render_node(node, req_path, path_relative, 0, &mut html);
    }
    html.push_str("</ul></nav>");
    html
}

fn render_node(
    node: &manager_list::Node,
    req_path: &str,
    path_relative: &str,
    depth: usize,
    out: &mut String,
) {
    let has_children = !node.children.is_empty();
    let li_class = if has_children { "project-node has-children" } else { "project-node" };
    let ul_class = format!("project-list level-{}", depth + 1);

    out.push_str(&format!(r#"<li class="{}">"#, li_class));

    let is_sel = is_selected(req_path, path_relative, &node.path);
    let a_class = if is_sel { "project-link selected" } else { "project-link" };

    // Always link to the project with a trailing slash for correct relative URL resolution
    let href = format!("{}/", node.path.trim_end_matches('/'));
    let href = html_escape(&href);
    let label = html_escape(&node.name);

    // Hook into client-side router:
    out.push_str(&format!(
        r#"<a class="{}" href="{}" onclick="return tm.handleLinkClick(event, this)">{}</a>"#,
        a_class, href, label
    ));

    if has_children {
        out.push_str(&format!(r#"<ul class="{}">"#, ul_class));
        for child in &node.children {
            render_node(child, req_path, path_relative, depth + 1, out);
        }
        out.push_str("</ul>");
    }
    out.push_str("</li>");
}

fn is_selected(req_path: &str, req_rel: &str, node_path: &str) -> bool {
    let req_path_norm = req_path.trim_end_matches('/');
    let req_rel_norm  = req_rel.trim_end_matches('/');
    let node_norm     = node_path.trim_end_matches('/');
    req_path_norm == node_norm || req_rel_norm == strip_projects_prefix(node_norm)
}

fn strip_projects_prefix(p: &str) -> &str {
    p.strip_prefix("/projects").unwrap_or(p)
}

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

fn fallback_template() -> String {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>{{TITLE}}</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>/* minimal fallback */</style>
</head>
<body>
  <div class="root">
    <div class="sidebar">{{SIDEBAR}}</div>
    <div class="content">{{CONTENT}}</div>
  </div>
</body>
</html>"#
        .to_string()
}
