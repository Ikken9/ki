use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::Path;

use ratatui::layout::{Position, Rect};

use crate::tree::flatten::{Flattened, flatten};
use crate::tree::item::TreeItem;

/// TODO
#[derive(Debug, Default, Clone, Eq)]
pub struct ExplorerState<Identifier>
where
    Identifier: AsRef<Path> + Clone + PartialEq + Eq + Hash + Debug,
{
    /// Contains the selected node
    pub selected: Vec<Identifier>,
    /// Contains the expanded nodes
    pub expanded: HashSet<Vec<Identifier>>,

    /// Helps tracking the state of the explorer, when `true` the explorer is open,
    /// when `false` it is closed
    pub open: bool,

    pub offset: usize,
    pub last_area: Rect,
    pub last_biggest_index: usize,
    /// All identifiers open on last render
    pub last_identifiers: Vec<Vec<Identifier>>,
    /// Identifier rendered at `y` on last render
    pub last_rendered_identifiers: Vec<(u16, Vec<Identifier>)>,
    pub ensure_selected_in_view_on_next_render: bool,
}

impl<Identifier> PartialEq for ExplorerState<Identifier>
where
    Identifier: AsRef<Path> + Clone + PartialEq + Eq + Hash + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
            && self.expanded.len() == other.expanded.len()
            && self
                .expanded
                .iter()
                .all(|item| other.expanded.contains(item))
            && self.selected == other.selected
            && self.ensure_selected_in_view_on_next_render
                == other.ensure_selected_in_view_on_next_render
            && self.last_area == other.last_area
            && self.last_biggest_index == other.last_biggest_index
            && self.last_identifiers == other.last_identifiers
            && self.last_rendered_identifiers == other.last_rendered_identifiers
            && self.open == other.open
    }
}

impl<Identifier> ExplorerState<Identifier>
where
    Identifier: AsRef<Path> + Clone + PartialEq + Eq + Hash + Debug,
{
    /// TODO
    pub const fn get_offset(&self) -> usize {
        self.offset
    }

    /// TODO
    pub const fn expanded(&self) -> &HashSet<Vec<Identifier>> {
        &self.expanded
    }

    /// Return the currently selected node
    #[allow(clippy::missing_const_for_fn)] // False positive
    pub fn selected(&self) -> &Vec<Identifier> {
        &self.selected
    }

    /// Get a flat list of all currently viewable (including by scrolling) [`TreeItem`]s with this `ExplorerState`.
    pub fn flatten<'text>(
        &self,
        items: Vec<TreeItem<'text, Identifier>>,
    ) -> Vec<Flattened<'text, Identifier>> {
        flatten(&self.expanded, items, &Vec::new())
    }

    /// Select the given identifier
    pub fn select(&mut self, identifier: Vec<Identifier>) -> bool {
        self.ensure_selected_in_view_on_next_render = true;
        let changed = self.selected != identifier;
        self.selected = identifier;
        changed
    }

    /// Expand a tree node.
    /// Returns `true` when it was collapsed and has been expanded.
    /// Returns `false` when it was already expanded.
    pub fn expand(&mut self, identifier: Vec<Identifier>) -> bool {
        if identifier.is_empty() {
            false
        } else {
            self.expanded.insert(identifier)
        }
    }

    /// Collapse a tree node.
    /// Returns `true` when it was expanded and has been collapsed.
    /// Returns `false` when it was already collapsed.
    pub fn collapse(&mut self, identifier: &Vec<Identifier>) -> bool {
        self.expanded.remove(identifier)
    }

    /// Toggles a tree node expanded/collapsed state with the given identifier.
    /// When it is currently expanded, then [`collapse`](Self::collapse) is called. Otherwise [`expand`](Self::expand).
    ///
    /// Returns `true` when a node is expanded / collapsed.
    /// As toggle always changes something, this only returns `false` when an empty identifier is given.
    pub fn toggle(&mut self, identifier: Vec<Identifier>) -> bool {
        if identifier.is_empty() {
            false
        } else if self.expanded.contains(&identifier) {
            self.collapse(&identifier)
        } else {
            self.expand(identifier)
        }
    }

    /// Toggles the currently selected tree node expanded/collapsed state.
    /// See also [`toggle`](Self::toggle)
    ///
    /// Returns `true` when a node is expanded / collapsed.
    /// As toggle always changes something, this only returns `false` when nothing is selected.
    pub fn toggle_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }

        self.ensure_selected_in_view_on_next_render = true;

        let was_expanded = self.expanded.remove(&self.selected);
        if was_expanded {
            return true;
        }

        self.expand(self.selected.clone())
    }

    /// Collapses all expanded nodes.
    ///
    /// Returns `true` when any node was closed.
    pub fn collapse_all(&mut self) -> bool {
        if self.expanded.is_empty() {
            false
        } else {
            self.expanded.clear();
            true
        }
    }

    /// Select the first node.
    ///
    /// Returns `true` when the selection changed.
    pub fn select_first(&mut self) -> bool {
        let identifier = self.last_identifiers.first().cloned().unwrap_or_default();
        self.select(identifier)
    }

    /// Select the last node.
    ///
    /// Returns `true` when the selection changed.
    pub fn select_last(&mut self) -> bool {
        let new_identifier = self.last_identifiers.last().cloned().unwrap_or_default();
        self.select(new_identifier)
    }

    /// TODO
    pub fn select_next(&mut self) -> bool {
        if self.last_identifiers.is_empty() {
            return false;
        }

        let current_pos = self
            .last_identifiers
            .iter()
            .position(|id| id == &self.selected);

        let new_pos = current_pos
            .map_or(0, |pos| pos.saturating_add(1))
            .min(self.last_biggest_index)
            .min(self.last_identifiers.len().saturating_sub(1));

        let new_identifier = match self.last_identifiers.get(new_pos) {
            Some(id) => id.clone(),
            None => {
                return false;
            }
        };

        let changed = self.selected != new_identifier;
        if changed {
            self.select(new_identifier)
        } else {
            false
        }
    }

    /// TODO
    pub fn select_prev(&mut self) -> bool {
        if self.last_identifiers.is_empty() {
            return false;
        }

        let current_pos = self
            .last_identifiers
            .iter()
            .position(|id| id == &self.selected);

        let new_pos = current_pos
            .map_or(usize::MAX, |pos| pos.saturating_sub(1))
            .min(self.last_biggest_index)
            .min(self.last_identifiers.len().saturating_sub(1));

        let new_identifier = match self.last_identifiers.get(new_pos) {
            Some(id) => id.clone(),
            None => {
                return false;
            }
        };

        let changed = self.selected != new_identifier;
        if changed {
            self.select(new_identifier)
        } else {
            false
        }
    }

    /// Get the identifier that was rendered for the given position on last render.
    pub fn rendered_at(&self, position: Position) -> Option<&[Identifier]> {
        if !self.last_area.contains(position) {
            return None;
        }

        self.last_rendered_identifiers
            .iter()
            .rev()
            .find(|(y, _)| position.y >= *y)
            .map(|(_, identifier)| identifier.as_ref())
    }

    /// Select what was rendered at the given position on last render.
    /// When it is already selected, toggle it.
    ///
    /// Returns `true` when the state changed.
    /// Returns `false` when there was nothing at the given position.
    pub fn click_at(&mut self, position: Position) -> bool {
        if let Some(identifier) = self.rendered_at(position) {
            if identifier == self.selected {
                self.toggle_selected()
            } else {
                self.select(identifier.to_vec())
            }
        } else {
            false
        }
    }

    /// Ensure the selected [`TreeItem`] is in view on next render
    pub fn scroll_selected_into_view(&mut self) {
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Scroll the specified amount of lines up
    ///
    /// Returns `true` when the scroll position changed.
    /// Returns `false` when the scrolling has reached the top.
    pub fn scroll_up(&mut self, lines: usize) -> bool {
        let before = self.offset;
        self.offset = self.offset.saturating_sub(lines);
        before != self.offset
    }

    /// Scroll the specified amount of lines down
    ///
    /// Returns `true` when the scroll position changed.
    /// Returns `false` when the scrolling has reached the last [`TreeItem`].
    pub fn scroll_down(&mut self, lines: usize) -> bool {
        let before = self.offset;
        self.offset = self
            .offset
            .saturating_add(lines)
            .min(self.last_biggest_index);
        before != self.offset
    }
}
