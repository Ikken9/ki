use core::hash::Hash;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::{Block, BorderType, Borders, StatefulWidgetRef, Widget};
use state::ExplorerState;
use std::fmt::Debug;
use std::path::Path;

use crate::fs::PathLike;
use crate::tree::{Tree, item::TreeItem};
use std::collections::BTreeSet;
use std::io;

pub mod state;

#[derive(Debug, Clone, PartialEq)]
pub struct Explorer<'text, T>
where
    T: AsRef<Path> + Clone + Eq + PartialEq + Ord,
{
    pub title: String,
    pub entries: BTreeSet<T>,
    pub root_path: T,
    pub tree: Tree<'text, T>,
}

impl<'text, T> Explorer<'text, T>
where
    T: PathLike + Clone + Eq + PartialEq + Ord + Debug,
{
    pub fn new(title: &str, root_path: &'text T) -> io::Result<Self> {
        // Create empty explorer first
        let explorer = Self {
            title: title.to_string(),
            entries: BTreeSet::new(),
            root_path: root_path.clone(),
            tree: Tree::new(vec![])?, // Start with empty tree
        };

        // This will be populated when add_entries is called
        Ok(explorer)
    }

    // Add a single entry to the entries map
    pub fn add_entry(&mut self, path: T) {
        self.entries.insert(path);
    }

    pub fn add_entries<I>(&mut self, entries: I) -> io::Result<()>
    where
        I: IntoIterator,
        I::Item: Into<T>,
    {
        self.entries.extend(entries.into_iter().map(Into::into));
        self.rebuild_tree()
    }

    /// Rebuild the tree based on the current entries
    pub fn rebuild_tree(&mut self) -> io::Result<()> {
        // Build tree starting from root path, but don't show root as an item
        let children = self
            .entries
            .iter()
            .filter(|p| p.as_ref().parent() == Some(self.root_path.as_ref()))
            .map(|path| {
                if path.is_dir() {
                    build_directory_tree(&self.root_path, path, &self.entries)
                } else {
                    Ok(TreeItem::new_leaf(
                        path.clone(),
                        path.as_ref()
                            .file_name()
                            .ok_or_else(|| {
                                io::Error::new(io::ErrorKind::InvalidData, "Path has no file name")
                            })?
                            .to_string_lossy()
                            .to_string(),
                    ))
                }
            })
            .collect::<io::Result<Vec<_>>>()?;

        self.tree = Tree::new(children)?;
        Ok(())
    }
}

impl<'text, T> StatefulWidgetRef for Explorer<'text, T>
where
    T: AsRef<Path> + Clone + Eq + PartialEq + Ord + Hash + Debug,
{
    type State = ExplorerState<T>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Store the area for navigation
        state.last_area = area;

        let title = format!(" {} ", self.title);
        let block = Block::default()
            .title(title)
            .italic()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let inner_area = block.inner(area);
        block.render(area, buf);

        // Render the tree directly modifying the state
        self.tree.render_ref(inner_area, buf, state);
    }
}

// Helper function to build a directory tree from paths
fn build_directory_tree<'a, T>(
    root_path: &T,
    current_path: &T,
    entries: &BTreeSet<T>,
) -> io::Result<TreeItem<'a, T>>
where
    T: PathLike,
{
    let mut children = Vec::new();

    for path in entries {
        if let Ok(rel_path) = path.as_ref().strip_prefix(current_path.as_ref()) {
            let components: Vec<_> = rel_path.components().collect();

            if components.len() == 1 {
                let component = components[0].as_os_str().to_string_lossy();
                let full_path = current_path.join(component.as_ref());

                if path.is_dir() {
                    let child = build_directory_tree(root_path, &full_path, entries)?;
                    children.push(child);
                } else {
                    children.push(TreeItem::new_leaf(full_path.clone(), component.to_string()));
                }
            }
        }
    }

    // Sort children (directories first, then files)
    children.sort_by(|a, b| {
        let a_is_dir = !a.children().is_empty();
        let b_is_dir = !b.children().is_empty();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.identifier().cmp(b.identifier()),
        }
    });

    let display_name = if current_path.as_ref() == root_path.as_ref() {
        "".to_string()
    } else {
        current_path
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    };

    TreeItem::new(current_path.clone(), display_name, children)
}
