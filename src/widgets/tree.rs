use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Paragraph, StatefulWidget, Widget},
};
use regex::Regex;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct TreeItem<'a> {
    contents: Text<'a>,
    color: Color,
    children: Vec<TreeItem<'a>>,
    expanded: bool,
}

impl<'a> TreeItem<'a> {
    pub fn new(contents: Text<'a>, color: Color, children: Vec<TreeItem<'a>>) -> Self {
        Self {
            contents,
            color,
            children,
            expanded: true,
        }
    }
}

#[derive(Debug, Clone)]
struct ComputedItem<'a> {
    item: &'a TreeItem<'a>,
    index: Vec<usize>,
    visible: bool,
    search_candidate: bool,
}

#[derive(Debug, Clone)]
pub struct TreeState<'a> {
    items: Vec<TreeItem<'a>>,
    position: usize,
    start: usize,
    end: usize,
    search: Option<Regex>,
}

impl<'a> TreeState<'a> {
    pub fn new(items: Vec<TreeItem<'a>>) -> Self {
        TreeState {
            items,
            position: Default::default(),
            start: Default::default(),
            end: Default::default(),
            search: Default::default(),
        }
    }

    pub fn position(&self) -> Option<Vec<usize>> {
        self.items()
            .iter()
            .filter(|item| item.visible)
            .nth(self.position)
            .map(|item| item.index.clone())
    }

    pub fn move_down(&mut self) {
        self.position = self
            .position
            .saturating_add(1)
            .min(self.items().iter().filter(|item| item.visible).count() - 1);
    }

    pub fn move_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    pub fn page_down(&mut self) {
        self.position = self
            .position
            .saturating_add(self.end - self.start - 1)
            .min(self.items().iter().filter(|item| item.visible).count() - 1);
    }

    pub fn page_up(&mut self) {
        self.position = self.position.saturating_sub(self.end - self.start - 1);
    }

    fn selected_mut(&mut self) -> Option<&mut TreeItem<'a>> {
        if let Some(index) = self.position() {
            let mut indidecs = index.into_iter();
            let mut item = self.items.get_mut(indidecs.next()?)?;
            for idx in indidecs {
                item = item.children.get_mut(idx)?;
            }
            Some(item)
        } else {
            None
        }
    }

    pub fn expand(&mut self) {
        if let Some(selected) = self.selected_mut() {
            selected.expanded = true;
        }
    }

    pub fn collapse(&mut self) {
        if let Some(selected) = self.selected_mut() {
            selected.expanded = false;
        }
    }

    pub fn search(&mut self, search: Option<&String>) -> Result<(), regex::Error> {
        self.search = if let Some(search) = search {
            Some(Regex::new(search)?)
        } else {
            None
        };
        Ok(())
    }

    fn items(&'a self) -> Vec<ComputedItem<'a>> {
        let mut to_flatten = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| (vec![index], true, item))
            .collect::<Vec<_>>();
        let mut entries = Vec::default();
        while let Some((index, visible, item)) = to_flatten.pop() {
            let search_candidate = if let Some(search) = &self.search {
                let text = item
                    .contents
                    .lines
                    .iter()
                    .flat_map(|line| line.spans.iter().map(|span| span.content.clone()))
                    .collect::<Vec<Cow<str>>>()
                    .join("");
                search.is_match(&text)
            } else {
                false
            };
            entries.push(ComputedItem {
                item,
                index: index.clone(),
                visible,
                search_candidate,
            });
            to_flatten.extend(
                item.children
                    .iter()
                    .enumerate()
                    .map(|(child_index, child)| {
                        (
                            index
                                .iter()
                                .cloned()
                                .chain(std::iter::once(child_index))
                                .collect(),
                            visible && item.expanded,
                            child,
                        )
                    })
                    .rev(),
            );
        }
        entries
    }

    fn update_bounds(&mut self, max_height: usize) {
        let heights = self
            .items()
            .into_iter()
            .filter(|item| item.visible)
            .scan(0, |acc, item| {
                *acc += item.item.contents.height();
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
                .unwrap_or(heights.len());
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
                .unwrap_or(heights.len());
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

        let inner_area = self.block.map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        state.update_bounds(inner_area.height as usize);

        let mut item_bottom = inner_area.top();
        for (item_idx, item) in state
            .items()
            .iter()
            .filter(|item| item.visible)
            .enumerate()
            .take(state.end)
            .skip(state.start)
        {
            let item_top = item_bottom;
            let indent = 2 * (item.index.len() as u16 - 1);
            let area = Rect::new(
                inner_area.left() + indent,
                item_top,
                inner_area.width - indent,
                item.item.contents.height() as u16,
            );
            let style = if item_idx == state.position && state.search.is_none() {
                Style::new()
                    .bg(item.item.color)
                    .add_modifier(Modifier::BOLD)
            } else if item.search_candidate {
                Style::new()
                    .fg(item.item.color)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::new().fg(item.item.color)
            };

            for (line_idx, line) in item.item.contents.lines.iter().enumerate() {
                let text_area = Rect::new(area.left(), item_top, line.width() as u16, 1);
                buf.set_style(text_area, style);
                buf.set_line(area.left(), item_top + line_idx as u16, line, area.width);
            }
            item_bottom += item.item.contents.height() as u16;
        }

        if let Some(search) = state.search.as_ref() {
            let search_area = Rect::new(area.left() + 1, area.bottom() - 1, area.width - 1, 1);
            let search_text = format!("\u{f002} {search}");
            let cursor_area = Rect::new(
                search_area.left() + search_text.chars().count() as u16,
                search_area.top(),
                1,
                1,
            );
            buf.set_style(cursor_area, Style::new().bg(Color::White));
            Paragraph::new(search_text).render(search_area, buf);
        }
    }
}
