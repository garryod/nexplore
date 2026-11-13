use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    symbols,
    text::Line,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Widget},
};

pub const HORIZONTAL_PIXELS_PER_COLUMN: usize = 2;

#[derive(Debug, Clone)]
pub struct Plot {
    title: String,
    points: Vec<(f64, f64)>,
}

impl Plot {
    pub fn new(title: impl Into<String>, points: Vec<(f64, f64)>) -> Self {
        Self {
            title: title.into(),
            points,
        }
    }
}

fn smooth_points(points: &[(f64, f64)], max_points: usize) -> Vec<(f64, f64)> {
    if points.len() <= max_points {
        return points.to_vec();
    }

    (0..max_points)
        .map(|bucket| {
            let start = bucket * points.len() / max_points;
            let end = (bucket + 1) * points.len() / max_points;
            let bucket_points = &points[start..end];
            let (x_sum, y_sum) = bucket_points
                .iter()
                .fold((0.0, 0.0), |(x_sum, y_sum), (x, y)| (x_sum + x, y_sum + y));
            let count = bucket_points.len() as f64;
            (x_sum / count, y_sum / count)
        })
        .collect()
}

impl Widget for Plot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let x_max = self
            .points
            .last()
            .map(|point| point.0)
            .unwrap_or(1.0)
            .max(1.0);
        let max_points = usize::from(area.width)
            .saturating_mul(HORIZONTAL_PIXELS_PER_COLUMN)
            .max(1);
        let points = smooth_points(&self.points, max_points);
        let (mut y_min, mut y_max) = points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), point| {
                (min.min(point.1), max.max(point.1))
            });

        if !y_min.is_finite() || !y_max.is_finite() {
            y_min = 0.0;
            y_max = 1.0;
        } else if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        Chart::new(vec![
            Dataset::default()
                .graph_type(GraphType::Line)
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Green))
                .data(&points),
        ])
        .block(Block::default().title(self.title).borders(Borders::TOP))
        .x_axis(
            Axis::default()
                .bounds([0.0, x_max])
                .labels([Line::from("0"), Line::from(format!("{x_max:.0}"))]),
        )
        .y_axis(Axis::default().bounds([y_min, y_max]).labels([
            Line::from(format!("{y_min:.2}")),
            Line::from(format!("{y_max:.2}")),
        ]))
        .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::smooth_points;

    #[test]
    fn smoothing_leaves_short_series_unchanged() {
        let points = vec![(0.0, 1.0), (1.0, 2.0)];

        assert_eq!(smooth_points(&points, 4), points);
    }

    #[test]
    fn smoothing_averages_series_into_requested_number_of_buckets() {
        let points = (0..8)
            .map(|value| (f64::from(value), f64::from(value)))
            .collect::<Vec<_>>();

        assert_eq!(
            smooth_points(&points, 4),
            vec![(0.5, 0.5), (2.5, 2.5), (4.5, 4.5), (6.5, 6.5)]
        );
    }
}
