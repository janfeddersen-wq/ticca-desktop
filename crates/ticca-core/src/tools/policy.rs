//! Tool execution policy limits

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToolPolicy {
    pub allowed_roots: Vec<PathBuf>,
    pub max_read_bytes: Option<u64>,
    pub max_write_bytes: Option<u64>,
}

impl ToolPolicy {
    pub fn allow_all() -> Self {
        Self {
            allowed_roots: Vec::new(),
            max_read_bytes: None,
            max_write_bytes: None,
        }
    }

    pub fn allow_root(root: PathBuf) -> Self {
        Self {
            allowed_roots: vec![root],
            max_read_bytes: None,
            max_write_bytes: None,
        }
    }

    pub fn is_path_allowed(&self, path: &Path) -> bool {
        if self.allowed_roots.is_empty() {
            return true;
        }

        self.allowed_roots.iter().any(|root| path.starts_with(root))
    }
}
