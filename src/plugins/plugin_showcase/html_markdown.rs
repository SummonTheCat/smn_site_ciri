use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};
use std::fmt::Write as _;

/// Convert Markdown to HTML wrapped with classes for styling:
/// - Container: <div class="md"> ... </div>
/// - Headings: <h1 class="md-h1">, ..., <h6 class="md-h6">
/// - Paragraph: <p class="md-p">
/// - Lists: <ul class="md-ul">, <ol class="md-ol">, <li class="md-li">
/// - Code: <pre class="md-pre"><code class="md-code language-xxx">...</code></pre>
/// - Inline code: <code class="md-code-inline">
/// - Links: <a class="md-a" ...>
/// - Images: <img class="md-img" ...>
/// - Blockquote: <blockquote class="md-blockquote">
/// - HR: <hr class="md-hr"/>
/// - Tables (enabled): <table class="md-table"> ...
pub fn render_markdown(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(md, opts);

    let mut out = String::with_capacity(md.len() + 256);
    out.push_str(r#"<div class="md">"#);

    for ev in parser {
        match ev {
            Event::Start(tag) => start_tag(tag, &mut out),
            Event::End(tag_end) => end_tag(tag_end, &mut out),
            Event::Text(text) => {
                escape_html(&mut out, &text);
            }
            Event::Code(text) => {
                out.push_str(r#"<code class="md-code-inline">"#);
                escape_html(&mut out, &text);
                out.push_str("</code>");
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                // For safety, treat raw HTML as text. Change to push verbatim if you trust sources.
                escape_html(&mut out, &html);
            }
            Event::FootnoteReference(name) => {
                out.push_str(r#"<sup class="md-footnote-ref">"#);
                escape_html(&mut out, &name);
                out.push_str("</sup>");
            }
            Event::SoftBreak => out.push('\n'),
            Event::HardBreak => out.push_str(r#"<br class="md-br"/>"#),
            Event::Rule => out.push_str(r#"<hr class="md-hr"/>"#),
            Event::TaskListMarker(checked) => {
                let attr = if checked { r#" checked="checked""# } else { "" };
                out.push_str(r#"<input class="md-task" type="checkbox" disabled="disabled""#);
                out.push_str(attr);
                out.push_str(r#"/>"#);
            }
        }
    }

    out.push_str("</div>");
    out
}

fn start_tag(tag: Tag, out: &mut String) {
    match tag {
        Tag::Paragraph => out.push_str(r#"<p class="md-p">"#),
        Tag::Heading { level, .. } => {
            let level_num = level as u8;
            let cls = match level_num {
                1 => "md-h1",
                2 => "md-h2",
                3 => "md-h3",
                4 => "md-h4",
                5 => "md-h5",
                _ => "md-h6",
            };
            write!(out, r#"<h{} class="{}">"#, level_num, cls).ok();
        }
        Tag::BlockQuote => out.push_str(r#"<blockquote class="md-blockquote">"#),
        Tag::CodeBlock(kind) => {
            let lang = match kind {
                CodeBlockKind::Indented => None,
                CodeBlockKind::Fenced(lang) => {
                    let l = lang.trim();
                    if l.is_empty() { None } else { Some(l.to_owned()) }
                }
            };
            match lang {
                Some(ref l) => write!(out, r#"<pre class="md-pre"><code class="md-code language-{}">"#, attr_escape(l)).ok(),
                None => {
                    out.push_str(r#"<pre class="md-pre"><code class="md-code">"#);
                    Some(())
                },
            };
        }
        Tag::List(Some(_start)) => out.push_str(r#"<ol class="md-ol">"#),
        Tag::List(None) => out.push_str(r#"<ul class="md-ul">"#),
        Tag::Item => out.push_str(r#"<li class="md-li">"#),
        Tag::Emphasis => out.push_str(r#"<em class="md-em">"#),
        Tag::Strong => out.push_str(r#"<strong class="md-strong">"#),
        Tag::Strikethrough => out.push_str(r#"<del class="md-del">"#),
        Tag::Link { link_type: _lt, dest_url, title, id: _ } => {
            // Add classes and safe target/rel for absolute URLs
            let d = dest_url.to_string();
            let (target, rel) = if is_abs_url(&d) {
                (r#" target="_blank""#, r#" rel="noopener noreferrer""#)
            } else {
                ("", "")
            };
            out.push_str(r#"<a class="md-a" href=""#);
            attr_escape_to(out, &d);
            out.push('"');
            if !title.is_empty() {
                out.push_str(r#" title=""#);
                attr_escape_to(out, &title);
                out.push('"');
            }
            out.push_str(target);
            out.push_str(rel);
            out.push('>');
        }
        Tag::Image { link_type: _lt, title, dest_url, id: _ } => {
            out.push_str(r#"<img class="md-img" src=""#);
            attr_escape_to(out, &dest_url);
            out.push('"');
            if !title.is_empty() {
                out.push_str(r#" title=""#);
                attr_escape_to(out, &title);
                out.push('"');
            }
            out.push_str(r#" alt="""#); // actual alt text will come via Text before End(Image)
        }
        Tag::Table(_alignments) => out.push_str(r#"<table class="md-table">"#),
        Tag::TableHead => out.push_str("<thead>"),
        Tag::TableRow => out.push_str("<tr>"),
        Tag::TableCell => out.push_str("<td>"),
        _ => {}
    }
}

fn end_tag(tag: pulldown_cmark::TagEnd, out: &mut String) {
    use pulldown_cmark::TagEnd;
    match tag {
        TagEnd::Paragraph => out.push_str("</p>"),
        TagEnd::Heading(level) => {
            let _ = write!(out, "</h{}>", level as u8);
        },
        TagEnd::BlockQuote => out.push_str("</blockquote>"),
        TagEnd::CodeBlock => out.push_str("</code></pre>"),
        TagEnd::List(true) => out.push_str("</ol>"),
        TagEnd::List(false) => out.push_str("</ul>"),
        TagEnd::Item => out.push_str("</li>"),
        TagEnd::Emphasis => out.push_str("</em>"),
        TagEnd::Strong => out.push_str("</strong>"),
        TagEnd::Strikethrough => out.push_str("</del>"),
        TagEnd::Link => out.push_str("</a>"),
        TagEnd::Image => out.push_str(r#""" />"#),
        TagEnd::Table => out.push_str("</table>"),
        TagEnd::TableHead => out.push_str("</thead>"),
        TagEnd::TableRow => out.push_str("</tr>"),
        TagEnd::TableCell => out.push_str("</td>"),
        _ => {}
    }
}

fn is_abs_url(s: &str) -> bool {
    let ss = s.to_ascii_lowercase();
    ss.starts_with("http://") || ss.starts_with("https://")
}

// --- escaping helpers ---

fn escape_html(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

fn attr_escape_to(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

fn attr_escape(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    attr_escape_to(&mut buf, s);
    buf
}
