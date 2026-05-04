use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AssetRoot {
    pub path: PathBuf,
}

impl AssetRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}
