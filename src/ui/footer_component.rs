use crate::portwatch::ExportFormat;
use chrono::{DateTime, Utc};

use crate::ui::theme::TableColors;

use ratatui::prelude::Color;
use ratatui::widgets::{Block, BorderType};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    prelude::Style,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Debug)]
pub struct FooterComponent {
    pub display: bool,
}
impl Default for FooterComponent {
    fn default() -> Self {
        Self { display: false }
    }
}

impl FooterComponent {
    pub fn toggle(&mut self) {
        self.display = !self.display;
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        colors: &TableColors,
        export_format: ExportFormat,
        is_tracking: bool,
        started_time: Option<DateTime<Utc>>,
        events_count: usize,
    ) {
        let started_str = started_time
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "-".into());

        let footer_text = if is_tracking {
            Line::from(vec![
                Span::styled(format!("{} changes", events_count), Style::default()),
                Span::raw(" since "),
                Span::styled(started_str, Style::default()),
                Span::raw(" | Format: "),
                Span::styled(
                    format!("{:?}", export_format),
                    Style::default().fg(colors.footer_border_color),
                ),
                Span::raw(" | "),
                Span::styled("[F]", Style::default()),
                Span::raw(" Format  "),
                Span::styled("[E]", Style::default()),
                Span::raw(" Export  "),
                Span::styled("[S]", Style::default()),
                Span::raw(" Stop"),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "Monitoring paused",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::raw(" — Press "),
                Span::styled("[S]", Style::default().fg(Color::Green)),
                Span::raw(" to start auditing"),
            ])
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(colors.row_fg).bg(colors.buffer_bg))
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(colors.footer_border_color))
                    .title("Auditing"),
            );

        frame.render_widget(footer, area);
    }
}
