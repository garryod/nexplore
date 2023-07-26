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

#[derive(Debug, Clone)]
pub struct TreeState<'a> {
    items: TreeItems<'a>,
    position: usize,
    start: usize,
    end: usize,
}

impl<'a> TreeState<'a> {
    pub fn new(items: TreeItems<'a>) -> Self {
        TreeState {
            items,
            position: Default::default(),
            start: Default::default(),
            end: Default::default(),
        }
    }

    pub fn position(&self) -> Option<Vec<usize>> {
        self.items.get(self.position).map(|item| item.index.clone())
    }

    pub fn move_down(&mut self) {
        self.position = self.position.saturating_add(1).min(self.items.len() - 1);
    }

    pub fn move_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    pub fn page_down(&mut self) {
        self.position = self
            .position
            .saturating_add(self.end - self.start - 1)
            .min(self.items.len() - 1);
    }

    pub fn page_up(&mut self) {
        self.position = self.position.saturating_sub(self.end - self.start - 1);
    }

    fn update_bounds(&mut self, max_height: usize) {
        let heights = self
            .items
            .iter()
            .scan(0, |acc, item| {
                *acc += item.contents.height();
                Some(*acc)
            })
            .collect::<Vec<_>>();

        if self.position < self.start {
            self.start = self.position;
            self.end = heights
                .iter()
                .enumerate()
                .find_map(|(idx, &height)| {
                    (heights[self.start] + max_height <= height).then_some(idx)
                })
                .unwrap_or(self.items.len());
        } else if heights[self.start] + max_height <= heights[self.position] {
            self.end = self.position + 1;
            self.start = heights
                .iter()
                .enumerate()
                .rev()
                .find_map(|(idx, height)| {
                    (height + max_height <= heights[self.end - 1]).then_some(idx + 1)
                })
                .unwrap_or(0);
        } else {
            self.end = heights
                .iter()
                .enumerate()
                .find_map(|(idx, &height)| {
                    (heights[self.start] + max_height <= height).then_some(idx)
                })
                .unwrap_or(self.items.len());
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tree<'i> {
    style: Style,
    block: Option<Block<'i>>,
}

impl<'i> Tree<'i> {
    #[must_use]
    pub fn block(mut self, block: Block<'i>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> StatefulWidget for Tree<'a> {
    type State = TreeState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        let area = self.block.clone().map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        state.update_bounds(area.height as usize);

        let mut item_bottom = area.top();
        for (item_idx, item) in state
            .items
            .iter()
            .enumerate()
            .take(state.end)
            .skip(state.start)
        {
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
