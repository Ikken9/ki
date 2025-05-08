use core::hash::Hash;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::Path;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{
    Block, Scrollbar, ScrollbarState, StatefulWidget, StatefulWidgetRef, Widget,
};
use unicode_width::UnicodeWidthStr as _;

use super::tree::flatten::Flattened;
use crate::explorer::state::ExplorerState;
use crate::tree::item::TreeItem;

pub(super) mod flatten;
pub(super) mod item;

/// TODO
#[derive(Debug, Clone, PartialEq)]
pub struct Tree<'text, Identifier> {
    items: Vec<TreeItem<'text, Identifier>>,

    /// Explorer block
    block: Option<Block<'static>>,
    scrollbar: Option<Scrollbar<'static>>,

    /// Style used as a base style for the widget
    style: Style,
    /// Style used to render selected item
    highlight_style: Style,
    /// Symbol in front of the selected item (shift all items to the right)
    highlight_symbol: String,

    /// Symbol displayed in front of a closed node (as in the children are currently not visible)
    node_closed_symbol: String,
    /// Symbol displayed in front of an open node (as in the children are currently visible)
    node_open_symbol: String,
    /// Symbol displayed in front of a node without children.
    node_no_children_symbol: String,
}

impl<'text, Identifier> Tree<'text, Identifier>
where
    Identifier: Clone + PartialEq + Eq + Hash,
{
    /// Create a new `Tree` that owns its items.
    ///
    /// # Errors
    ///
    /// Errors when there are duplicate identifiers in the items.
    pub fn new(items: Vec<TreeItem<'text, Identifier>>) -> std::io::Result<Self> {
        let identifiers = items
            .iter()
            .map(|item| &item.identifier)
            .collect::<HashSet<_>>();
        if identifiers.len() != items.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "The items contain duplicate identifiers",
            ));
        }

        Ok(Self {
            items,
            block: None,
            scrollbar: None,
            style: Style::new(),
            highlight_style: Style::new().add_modifier(Modifier::REVERSED),
            highlight_symbol: String::new(),
            node_closed_symbol: "\u{25b6} ".to_string(),
            node_open_symbol: "\u{25bc} ".to_string(),
            node_no_children_symbol: "  ".to_string(),
        })
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Show the scrollbar when rendering this widget.
    pub fn experimental_scrollbar(mut self, scrollbar: Option<Scrollbar<'static>>) -> Self {
        self.scrollbar = scrollbar;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &str) -> Self {
        self.highlight_symbol = highlight_symbol.to_string();
        self
    }

    pub fn node_closed_symbol(mut self, symbol: &str) -> Self {
        self.node_closed_symbol = symbol.to_string();
        self
    }

    pub fn node_open_symbol(mut self, symbol: &str) -> Self {
        self.node_open_symbol = symbol.to_string();
        self
    }

    pub fn node_no_children_symbol(mut self, symbol: &str) -> Self {
        self.node_no_children_symbol = symbol.to_string();
        self
    }

    pub fn items(&self) -> &Vec<TreeItem<Identifier>> {
        &self.items
    }
}

impl<Identifier> StatefulWidgetRef for Tree<'_, Identifier>
where
    Identifier: AsRef<Path> + Clone + PartialEq + Eq + Hash + Debug,
{
    type State = ExplorerState<Identifier>;

    #[allow(clippy::too_many_lines)]
    fn render_ref(&self, full_area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(full_area, self.style);

        // Get the inner area inside a possible block, otherwise use the full area
        let area = self.block.clone().map_or(full_area, |block| {
            let inner_area = block.inner(full_area);
            block.render(full_area, buf);
            inner_area
        });

        state.last_area = area;
        state.last_rendered_identifiers.clear();
        if area.width < 1 || area.height < 1 {
            return;
        }

        let visible = state.flatten(self.items.clone());
        state.last_biggest_index = visible.len().saturating_sub(1);
        if visible.is_empty() {
            return;
        }

        let available_height = area.height as usize;

        let ensure_index_in_view =
            if state.ensure_selected_in_view_on_next_render && !state.selected.is_empty() {
                visible
                    .iter()
                    .position(|flattened| flattened.identifier == state.selected)
            } else {
                None
            };

        // Ensure last line is still visible
        let mut start = state.offset.min(state.last_biggest_index);

        if let Some(ensure_index_in_view) = ensure_index_in_view {
            start = start.min(ensure_index_in_view);
        }

        let mut end = start;
        let mut height = 0;
        for item_height in visible
            .iter()
            .skip(start)
            .map(|flattened| flattened.item.height())
        {
            if height + item_height > available_height {
                break;
            }
            height += item_height;
            end += 1;
        }

        if let Some(ensure_index_in_view) = ensure_index_in_view {
            while ensure_index_in_view >= end {
                height += visible[end].item.height();
                end += 1;
                while height > available_height {
                    height = height.saturating_sub(visible[start].item.height());
                    start += 1;
                }
            }
        }

        state.offset = start;
        state.ensure_selected_in_view_on_next_render = false;

        if let Some(scrollbar) = self.scrollbar.clone() {
            let mut scrollbar_state = ScrollbarState::new(visible.len().saturating_sub(height))
                .position(start)
                .viewport_content_length(height);
            let scrollbar_area = Rect {
                // Inner height to be exactly as the content
                y: area.y,
                height: area.height,
                // Outer width to stay on the right border
                x: full_area.x,
                width: full_area.width,
            };
            scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);
        }

        let blank_symbol = " ".repeat(self.highlight_symbol.width());

        let mut current_height = 0;
        let has_selection = !state.selected.is_empty();
        #[allow(clippy::cast_possible_truncation)]
        for flattened in visible.iter().skip(state.offset).take(end - start) {
            let Flattened { identifier, item } = flattened;
            let x = area.x;
            let y = area.y + current_height;
            let height = item.height() as u16;
            current_height += height;

            let area = Rect {
                x,
                y,
                width: area.width,
                height,
            };

            let text = &item.text;
            let item_style = text.style;

            let is_selected = state.selected == *identifier;
            let after_highlight_symbol_x = if has_selection {
                let symbol = if is_selected {
                    &self.highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, area.width as usize, item_style);
                x
            } else {
                x
            };

            let after_depth_x = {
                let indent_width = flattened.depth() * 2;
                let (after_indent_x, _) = buf.set_stringn(
                    after_highlight_symbol_x,
                    y,
                    " ".repeat(indent_width),
                    indent_width,
                    item_style,
                );
                let symbol = if item.children.is_empty() {
                    &self.node_no_children_symbol
                } else if state.expanded.contains(identifier) {
                    &self.node_open_symbol
                } else {
                    &self.node_closed_symbol
                };
                let max_width = area.width.saturating_sub(after_indent_x - x);
                let (x, _) =
                    buf.set_stringn(after_indent_x, y, symbol, max_width as usize, item_style);
                x
            };

            let text_area = Rect {
                x: after_depth_x,
                width: area.width.saturating_sub(after_depth_x - x),
                ..area
            };
            text.render(text_area, buf);

            if is_selected {
                buf.set_style(area, self.highlight_style);
            }

            state
                .last_rendered_identifiers
                .push((area.y, identifier.clone()));
        }

        state.last_identifiers = visible
            .into_iter()
            .map(|flattened| flattened.identifier)
            .collect();
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;

    #[must_use]
    #[track_caller]
    fn render(width: u16, height: u16, state: &mut ExplorerState<&'static str>) -> Buffer {
        let items = TreeItem::example();
        let tree = Tree::new(items).unwrap();
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        StatefulWidgetRef::render_ref(&tree, area, &mut buffer, state);
        buffer
    }

    #[test]
    fn does_not_panic() {
        _ = render(0, 0, &mut ExplorerState::default());
        _ = render(10, 0, &mut ExplorerState::default());
        _ = render(0, 10, &mut ExplorerState::default());
        _ = render(10, 10, &mut ExplorerState::default());
    }

    #[test]
    fn nothing_open() {
        let buffer = render(10, 4, &mut ExplorerState::default());
        #[rustfmt::skip]
        let expected = Buffer::with_lines([
            "  Alfa    ",
            "▶ Bravo   ",
            "  Hotel   ",
            "          ",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn depth_one() {
        let mut state = ExplorerState::default();
        state.expand(vec!["b"]);
        let buffer = render(13, 7, &mut state);
        let expected = Buffer::with_lines([
            "  Alfa       ",
            "▼ Bravo      ",
            "    Charlie  ",
            "  ▶ Delta    ",
            "    Golf     ",
            "  Hotel      ",
            "             ",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn depth_two() {
        let mut state = ExplorerState::default();
        state.expand(vec!["b"]);
        state.expand(vec!["b", "d"]);
        let buffer = render(15, 9, &mut state);
        let expected = Buffer::with_lines([
            "  Alfa         ",
            "▼ Bravo        ",
            "    Charlie    ",
            "  ▼ Delta      ",
            "      Echo     ",
            "      Foxtrot  ",
            "    Golf       ",
            "  Hotel        ",
            "               ",
        ]);
        assert_eq!(buffer, expected);
    }
}
