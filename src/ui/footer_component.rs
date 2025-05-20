use crate::ApplicationMode;
use crate::explorer::ExportFormat;
use crate::ui::theme::TableColors;
use ratatui::Frame;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::prelude::Style;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

#[derive(Debug)]
pub struct FooterComponent {
    pub display: bool,
    pub export_format: ExportFormat,
}
impl Default for FooterComponent {
    fn default() -> Self {
        Self {
            display: false,
            export_format: ExportFormat::Json,
        }
    }
}

impl FooterComponent {
    pub fn toggle(&mut self) {
        self.display = !self.display;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, _colors: &TableColors, is_tracking: bool) {
        let footer_text = if is_tracking {
            Line::from(vec![
                Span::styled(
                    "🔴 Recording active",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(" — All port changes are being logged. "),
                Span::raw("Export format: "),
                Span::styled(format!("{:?}", self.export_format), Style::default()),
                Span::raw(" | Press "),
                Span::styled("[E]", Style::default()),
                Span::raw(" to export | "),
                Span::styled("[S]", Style::default()),
                Span::raw(" to stop"),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "🟡 Monitoring paused",
                    Style::default().add_modifier(Modifier::ITALIC),
                ),
                Span::raw(" — Press "),
                Span::styled("[S]", Style::default()),
                Span::raw(" to start recording"),
            ])
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default());

        frame.render_widget(footer, area);
    }
}
