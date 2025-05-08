use std::{cmp::Ordering, fmt::Debug, fs, hash::Hash, path::Path, path::PathBuf};

/// TODO
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct SortablePath(pub PathBuf);

impl SortablePath {
    pub fn is_dir(&self) -> bool {
        fs::metadata(&self.0).map(|m| m.is_dir()).unwrap_or(false)
    }
}

impl Ord for SortablePath {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_dir(), other.is_dir()) {
            // If both are directories or both are files, compare alphabetically
            (true, true) | (false, false) => self.0.cmp(&other.0),
            // Directories come before files
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
        }
    }
}

impl PartialOrd for SortablePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub trait PathLike: AsRef<Path> + Clone + Eq + PartialEq + Ord + Hash + Debug {
    fn is_dir(&self) -> bool;
    fn join<P: AsRef<Path>>(&self, path: P) -> Self;
}

impl PathLike for SortablePath {
    fn is_dir(&self) -> bool {
        fs::metadata(&self.0).map(|m| m.is_dir()).unwrap_or(false)
    }

    fn join<P: AsRef<Path>>(&self, path: P) -> Self {
        SortablePath(self.0.join(path))
    }
}

impl AsRef<Path> for SortablePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl PathLike for PathBuf {
    fn is_dir(&self) -> bool {
        self.as_path().is_dir()
    }

    fn join<P: AsRef<Path>>(&self, path: P) -> Self {
        self.as_path().join(path)
    }
}

impl From<PathBuf> for SortablePath {
    fn from(path: PathBuf) -> Self {
        SortablePath(path)
    }
}
