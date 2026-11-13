use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Clear, Row, Table, Widget},
};

const README: &str = include_str!("../../README.md");
const HELP_TITLE: &str = "Key Bindings (? / Esc to close)";
const COLUMN_SPACING: u16 = 2;

#[derive(Debug, Clone)]
struct HelpDialog {
    bindings: Vec<(String, String)>,
}

impl HelpDialog {
    fn new() -> Self {
        Self {
            bindings: readme_keybindings(),
        }
    }

    fn area(&self, area: Rect) -> Rect {
        let (action_width, binding_width) = self.column_widths();
        let content_width = action_width
            .saturating_add(usize::from(COLUMN_SPACING))
            .saturating_add(binding_width);
        let width = u16::try_from(content_width.max(HELP_TITLE.len()).saturating_add(2))
            .unwrap_or(u16::MAX)
            .min(area.width);
        let height = u16::try_from(self.bindings.len().saturating_add(2))
            .unwrap_or(u16::MAX)
            .min(area.height);
        Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        )
    }

    fn column_widths(&self) -> (usize, usize) {
        self.bindings.iter().fold(
            (0, 0),
            |(action_width, binding_width), (action, binding)| {
                (
                    action_width.max(action.len()),
                    binding_width.max(binding.len()),
                )
            },
        )
    }
}

impl Widget for HelpDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (action_width, _) = self.column_widths();
        let rows = self
            .bindings
            .into_iter()
            .map(|(action, binding)| Row::new(vec![Cell::from(action), Cell::from(binding)]));
        Clear.render(area, buf);
        Table::new(
            rows,
            [
                Constraint::Length(u16::try_from(action_width).unwrap_or(u16::MAX)),
                Constraint::Min(0),
            ],
        )
        .column_spacing(COLUMN_SPACING)
        .block(
            Block::default()
                .title(HELP_TITLE)
                .border_style(Style::new().fg(Color::Yellow))
                .borders(Borders::ALL),
        )
        .render(area, buf);
    }
}

pub fn render_help_dialog(frame: &mut Frame) {
    let dialog = HelpDialog::new();
    let area = dialog.area(frame.area());
    frame.render_widget(dialog, area);
}

fn readme_keybindings() -> Vec<(String, String)> {
    README
        .lines()
        .skip_while(|line| !line.starts_with("| Action"))
        .skip(2)
        .take_while(|line| line.starts_with("|"))
        .filter_map(|line| {
            let escaped = line.replace("\\|", "§");
            let mut columns = escaped
                .trim_matches('|')
                .split("|")
                .map(|column| column.trim().replace("§", "|"));
            Some((columns.next()?, columns.next()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{COLUMN_SPACING, HelpDialog, readme_keybindings};
    use ratatui::layout::Rect;

    #[test]
    fn help_dialog_reads_keybindings_from_readme() {
        let bindings = readme_keybindings();

        assert!(bindings.contains(&(String::from("Quit"), String::from("Esc | Q"))));
        assert!(bindings.contains(&(String::from("Help"), String::from("?"))));
    }

    #[test]
    fn help_dialog_is_wide_enough_for_both_columns() {
        let dialog = HelpDialog::new();
        let (action_width, binding_width) = dialog.column_widths();
        let area = dialog.area(Rect::new(0, 0, u16::MAX, u16::MAX));

        assert!(
            usize::from(area.width)
                >= action_width + usize::from(COLUMN_SPACING) + binding_width + 2
        );
    }
}
