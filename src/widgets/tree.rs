use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Text,
    widgets::{Block, StatefulWidget, Widget},
};

#[derive(Debug)]
pub struct TreeItem<'a> {
    contents: Text<'a>,
    children: Vec<TreeItem<'a>>,
}

impl<'a> TreeItem<'a> {
    pub fn new(contents: Text<'a>, children: Vec<TreeItem<'a>>) -> Self {
        Self { children, contents }
    }
}

#[derive(Debug)]
pub struct TreeState;

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

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        buf.set_style(area, self.style);

        let area = self.block.map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        let items = flatten(self.items);

        let mut item_bottom = area.top();
        for item in items.into_iter() {
            let item_top = item_bottom;
            if item_top + item.height() as u16 > area.bottom() {
                break;
            }
            let area = Rect::new(area.left(), item_top, area.width, item.height() as u16);
            buf.set_style(area, Style::default());

            for (line_idx, line) in item.lines.iter().enumerate() {
                buf.set_line(area.left(), item_top + line_idx as u16, line, area.width);
            }
            item_bottom += item.height() as u16;
        }
    }
}

fn flatten(mut items: Vec<TreeItem>) -> Vec<Text> {
    items.reverse();
    let mut entries = Vec::default();
    while let Some(item) = items.pop() {
        entries.push(item.contents);
        items.extend(item.children.into_iter().rev());
    }
    entries
}

impl<'a> Widget for Tree<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TreeState;
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
