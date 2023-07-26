use derive_more::Deref;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, StatefulWidget, Widget},
};

#[derive(Debug)]
pub struct TreeItem<'a> {
    contents: Text<'a>,
    color: Color,
    children: Vec<TreeItem<'a>>,
}

impl<'a> TreeItem<'a> {
    pub fn new(contents: Text<'a>, color: Color, children: Vec<TreeItem<'a>>) -> Self {
        Self {
            contents,
            color,
            children,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlatItem<'a> {
    index: Vec<usize>,
    contents: Text<'a>,
    color: Color,
}

#[derive(Debug, Clone, Deref)]
pub struct TreeItems<'i>(pub Vec<FlatItem<'i>>);

impl<'i> From<Vec<TreeItem<'i>>> for TreeItems<'i> {
    fn from(items: Vec<TreeItem<'i>>) -> Self {
        Self(flatten(items))
    }
}

#[derive(Debug, Default)]
pub struct TreeState {
    position: usize,
    offset: usize,
}

impl TreeState {
    pub fn position(&self, items: &TreeItems) -> Option<Vec<usize>> {
        items.get(self.position).map(|item| item.index.clone())
    }

    pub fn move_down(&mut self, items: &TreeItems) {
        self.position = self.position.saturating_add(1).min(items.len() - 1);
    }

    pub fn move_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }
}

#[derive(Debug, Clone)]
pub struct Tree<'i> {
    items: TreeItems<'i>,
    style: Style,
    block: Option<Block<'i>>,
}

impl<'i> Tree<'i> {
    #[must_use]
    pub fn new(items: TreeItems<'i>) -> Self {
        Self {
            items,
            style: Style::default(),
            block: None,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'i>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'i> Tree<'i> {
    fn bounds(&self, position: usize, prior_offset: usize, max_height: usize) -> (usize, usize) {
        let heights = self
            .items
            .iter()
            .scan(0, |acc, item| {
                *acc += item.contents.height();
                Some(*acc)
            })
            .collect::<Vec<_>>();

        if position < prior_offset {
            let start = position;
            let end = heights
                .iter()
                .enumerate()
                .find_map(|(idx, &height)| (heights[start] + max_height <= height).then_some(idx))
                .unwrap_or(self.items.len());
            (start, end)
        } else if heights[prior_offset] + max_height <= heights[position] {
            let end = position + 1;
            let start = heights
                .iter()
                .enumerate()
                .rev()
                .find_map(|(idx, height)| {
                    (height + max_height <= heights[end - 1]).then_some(idx + 1)
                })
                .unwrap_or(0);
            (start, end)
        } else {
            let start = prior_offset;
            let end = heights
                .iter()
                .enumerate()
                .find_map(|(idx, &height)| (heights[start] + max_height <= height).then_some(idx))
                .unwrap_or(self.items.len());
            (start, end)
        }
    }
}

impl<'a> StatefulWidget for Tree<'a> {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        let area = self.block.clone().map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        let (start, end) = self.bounds(state.position, state.offset, area.height as usize);
        state.offset = start;

        let mut item_bottom = area.top();
        for (item_idx, item) in self.items.iter().enumerate().take(end).skip(start) {
            let item_top = item_bottom;
            let indent = 2 * (item.index.len() as u16 - 1);
            let area = Rect::new(
                area.left() + indent,
                item_top,
                area.width - indent,
                item.contents.height() as u16,
            );
            let style = if item_idx == state.position {
                Style::new().bg(item.color).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(item.color)
            };
            buf.set_style(area, style);

            for (line_idx, line) in item.contents.lines.iter().enumerate() {
                buf.set_line(area.left(), item_top + line_idx as u16, line, area.width);
            }
            item_bottom += item.contents.height() as u16;
        }
    }
}

fn flatten(items: Vec<TreeItem>) -> Vec<FlatItem> {
    let mut to_flatten = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| (vec![index], item))
        .collect::<Vec<_>>();
    let mut entries = Vec::default();
    while let Some((index, item)) = to_flatten.pop() {
        entries.push(FlatItem {
            index: index.clone(),
            contents: item.contents,
            color: item.color,
        });
        to_flatten.extend(
            item.children
                .into_iter()
                .enumerate()
                .map(|(child_index, item)| {
                    (
                        index
                            .iter()
                            .cloned()
                            .chain(std::iter::once(child_index))
                            .collect(),
                        item,
                    )
                })
                .rev(),
        );
    }
    entries
}
