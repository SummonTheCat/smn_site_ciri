use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// The only node shape used in your file.
#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub children: Vec<Node>,
}

/// Matches your top-level JSON:
/// { "project_tree": [ { name, path, children }, ... ] }
#[derive(Debug, Deserialize)]
struct ProjectRoot {
    project_tree: Vec<Node>,
}

/// Simple native error type (no extra crates).
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { Error::Io(e) }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self { Error::Json(e) }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Json(e) => write!(f, "JSON parse error: {e}"),
        }
    }
}
impl std::error::Error for Error {}

/// Public handle for read-only navigation.
#[derive(Debug, Clone)]
pub struct ProjectStructure {
    roots: Vec<Node>,
}

/// Load the structure from a file path.
pub fn get_project_structure<P: AsRef<Path>>(path: P) -> Result<ProjectStructure, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let parsed: ProjectRoot = serde_json::from_reader(reader)?;
    Ok(ProjectStructure { roots: parsed.project_tree })
}

impl ProjectStructure {
    /// Root nodes.
    pub fn roots(&self) -> &[Node] {
        &self.roots
    }

    /// Total node count.
    pub fn count(&self) -> usize {
        self.roots.iter().map(count_nodes).sum()
    }

    /// Find a node by exact `path` (depth-first).
    pub fn find_by_path(&self, target_path: &str) -> Option<&Node> {
        for r in &self.roots {
            if let Some(hit) = find_by_path(r, target_path) {
                return Some(hit);
            }
        }
        None
    }

    /// Depth-first iterator over all nodes (read-only).
    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        DfsIter::new(&self.roots)
    }
}

/// ---- internal helpers ----

fn count_nodes(n: &Node) -> usize {
    1 + n.children.iter().map(count_nodes).sum::<usize>()
}

fn find_by_path<'a>(n: &'a Node, target: &str) -> Option<&'a Node> {
    if n.path == target {
        return Some(n);
    }
    for c in &n.children {
        if let Some(hit) = find_by_path(c, target) {
            return Some(hit);
        }
    }
    None
}

/// Simple DFS iterator over a forest.
struct DfsIter<'a> {
    stack: Vec<&'a Node>,
}
impl<'a> DfsIter<'a> {
    fn new(roots: &'a [Node]) -> Self {
        let mut stack = Vec::with_capacity(roots.len());
        for n in roots.iter().rev() {
            stack.push(n);
        }
        Self { stack }
    }
}
impl<'a> Iterator for DfsIter<'a> {
    type Item = &'a Node;
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        for c in node.children.iter().rev() {
            self.stack.push(c);
        }
        Some(node)
    }
}
