use std::collections::HashSet;

use ratatui::text::Text;

/// TODO
#[derive(Debug, Clone, PartialEq)]
pub struct TreeItem<'text, Identifier> {
    pub identifier: Identifier,
    pub text: Text<'text>,
    pub children: Vec<Self>,
}

impl<'text, Identifier> TreeItem<'text, Identifier>
where
    Identifier: Clone + PartialEq + Eq + core::hash::Hash,
{
    /// Create a new `TreeItem` without children.
    pub fn new_leaf<T>(identifier: Identifier, text: T) -> Self
    where
        T: Into<Text<'text>>,
    {
        Self {
            identifier,
            text: text.into(),
            children: Vec::new(),
        }
    }

    /// Create a new `TreeItem` with children.
    ///
    /// # Errors
    ///
    /// Errors when there are duplicate identifiers in the children.
    pub fn new<T>(identifier: Identifier, text: T, children: Vec<Self>) -> std::io::Result<Self>
    where
        T: Into<Text<'text>>,
    {
        let identifiers = children
            .iter()
            .map(|item| &item.identifier)
            .collect::<HashSet<_>>();
        if identifiers.len() != children.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "The children contain duplicate identifiers",
            ));
        }

        Ok(Self {
            identifier,
            text: text.into(),
            children,
        })
    }

    /// Get a reference to the identifier.
    pub const fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    #[allow(clippy::missing_const_for_fn)] // False positive
    pub fn children(&self) -> &[Self] {
        &self.children
    }

    /// Get a reference to a child by index.
    pub fn child(&self, index: usize) -> Option<&Self> {
        self.children.get(index)
    }

    /// Get a mutable reference to a child by index.
    ///
    /// When you choose to change the `identifier` the [`TreeState`](crate::TreeState) might not work as expected afterwards.
    pub fn child_mut(&mut self, index: usize) -> Option<&mut Self> {
        self.children.get_mut(index)
    }

    pub fn height(&self) -> usize {
        self.text.height()
    }

    /// Add a child to the `TreeItem`.
    ///
    /// # Errors
    ///
    /// Errors when the `identifier` of the `child` already exists in the children.
    pub fn add_child(&mut self, child: Self) -> std::io::Result<()> {
        let existing = self
            .children
            .iter()
            .map(|item| &item.identifier)
            .collect::<HashSet<_>>();
        if existing.contains(&child.identifier) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "identifier already exists in the children",
            ));
        }

        self.children.push(child);
        Ok(())
    }
}

impl TreeItem<'static, &'static str> {
    #[cfg(test)]

    pub(crate) fn example() -> Vec<Self> {
        vec![
            Self::new_leaf("a", "Alfa"),
            Self::new(
                "b",
                "Bravo",
                vec![
                    Self::new_leaf("c", "Charlie"),
                    Self::new(
                        "d",
                        "Delta",
                        vec![Self::new_leaf("e", "Echo"), Self::new_leaf("f", "Foxtrot")],
                    )
                    .expect("all item identifiers are unique"),
                    Self::new_leaf("g", "Golf"),
                ],
            )
            .expect("all item identifiers are unique"),
            Self::new_leaf("h", "Hotel"),
        ]
    }
}

#[test]
#[should_panic = "duplicate identifiers"]
fn tree_item_new_errors_with_duplicate_identifiers() {
    let item = TreeItem::new_leaf("same", "text");
    let another = item.clone();
    TreeItem::new("root", "Root", vec![item, another]).unwrap();
}

#[test]
#[should_panic = "identifier already exists"]
fn tree_item_add_child_errors_with_duplicate_identifiers() {
    let item = TreeItem::new_leaf("same", "text");
    let another = item.clone();
    let mut root = TreeItem::new("root", "Root", vec![item]).unwrap();
    root.add_child(another).unwrap();
}
