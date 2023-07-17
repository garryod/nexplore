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

#[derive(Debug)]
struct FlatItem<'a> {
    index: Vec<usize>,
    contents: Text<'a>,
    color: Color,
}

#[derive(Debug, Default)]
pub struct TreeState {
    position: usize,
}

impl TreeState {
    pub fn move_down(&mut self) {
        self.position = self.position.saturating_add(1);
    }

    pub fn move_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }
}

#[derive(Debug)]
pub struct Tree<'a> {
    items: Vec<TreeItem<'a>>,
    style: Style,
    block: Option<Block<'a>>,
}

impl<'a> Tree<'a> {
    #[must_use]
    pub fn new(items: Vec<TreeItem<'a>>) -> Self {
        Self {
            items,
            style: Style::default(),
            block: None,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> StatefulWidget for Tree<'a> {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        let area = self.block.map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        let items = flatten(self.items);

        let mut item_bottom = area.top();
        for (item_idx, item) in items.into_iter().enumerate() {
            let item_top = item_bottom;
            if item_top + item.contents.height() as u16 > area.bottom() {
                break;
            }
            let indent = 2 * (item.index.len() as u16 - 1);
            let area = Rect::new(
                area.left() + indent,
                item_top,
                area.width - indent,
                item.contents.height() as u16,
            );
            let style = if item_idx == state.position {
                Style::new().bg(Color::White).add_modifier(Modifier::BOLD)
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
