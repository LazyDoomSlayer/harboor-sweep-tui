use crate::ui::theme::TableColors;

use crate::explorer::ExportFormat;
use crate::util::popup_area;
use ratatui::text::{Span, Text};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    prelude::Style,
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportAction {
    Export,
    Cancel,
}

/// A component that handles the snapshots
#[derive(Debug)]
pub struct SnapshotsComponent {
    /// Whether the popup is displayed
    pub display: bool,
    pub action: ExportAction,
    pub selected_format: ExportFormat,
}

impl Default for SnapshotsComponent {
    fn default() -> Self {
        Self {
            display: false,
            action: ExportAction::Export,
            selected_format: ExportFormat::Json,
        }
    }
}

impl ExportFormat {
    pub fn next(self) -> Self {
        match self {
            ExportFormat::Json => ExportFormat::Csv,
            ExportFormat::Csv => ExportFormat::Yaml,
            ExportFormat::Yaml => ExportFormat::Json,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ExportFormat::Json => ExportFormat::Yaml,
            ExportFormat::Csv => ExportFormat::Json,
            ExportFormat::Yaml => ExportFormat::Csv,
        }
    }
}

impl SnapshotsComponent {
    /// Toggle display on/off, clear selection when opening
    pub fn toggle(&mut self) {
        self.display = !self.display;
    }

    pub fn render_radio(&self, label: &str, selected: bool, colors: &TableColors) -> Vec<Span<'_>> {
        let symbol = if selected { "[x]" } else { "[ ]" };

        let symbol_style = if selected {
            Style::default().fg(colors.footer_border_color)
        } else {
            Style::default()
        };

        vec![
            Span::styled(symbol, symbol_style),
            Span::raw(format!(" {}", label)),
        ]
    }
    /// Select the next format
    pub fn next_format(&mut self) {
        self.selected_format = self.selected_format.next();
    }
    /// Select the previous format   
    pub fn prev_format(&mut self) {
        self.selected_format = self.selected_format.prev();
    }

    /// Renders the popup
    pub fn render(&self, frame: &mut Frame, area: Rect, colors: &TableColors) {
        if !self.display {
            return;
        }

        let block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::new().fg(colors.footer_border_color))
            .bg(colors.buffer_bg)
            .title(" Snapshotting ");

        let area = popup_area(area, 4, 5);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Length(4),
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);

        // 1) Explanation snapshot
        let explanation = Paragraph::new(Line::from("Takes a snapshot of all active listening ports and their processes. This snapshot can be exported for later comparison, auditing, or diagnostics."))
            .style(Style::default().fg(colors.row_fg).bg(colors.buffer_bg))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(
            explanation,
            chunks[1].inner(Margin {
                horizontal: 2,
                vertical: 0,
            }),
        );

        // 2) Checkboxes
        let lines = vec![
            Line::from("Export Format:"),
            Line::from(self.render_radio(
                "JSON",
                self.selected_format == ExportFormat::Json,
                colors,
            )),
            Line::from(self.render_radio("CSV", self.selected_format == ExportFormat::Csv, colors)),
            Line::from(self.render_radio(
                "YAML",
                self.selected_format == ExportFormat::Yaml,
                colors,
            )),
        ];

        let paragraph =
            Paragraph::new(Text::from(lines)).style(Style::default().bg(colors.buffer_bg));

        frame.render_widget(
            paragraph,
            chunks[3].inner(Margin {
                horizontal: 2,
                vertical: 0,
            }),
        );

        // 3) buttons
        let buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
            .flex(Flex::Center)
            .split(chunks[5]);

        let export_btn = Paragraph::new("Export")
            .alignment(ratatui::layout::Alignment::Center)
            .block(if self.action == ExportAction::Export {
                Block::bordered().border_style(Style::new().fg(colors.selected_cell_style_fg))
            } else {
                Block::bordered().border_style(Style::new().fg(colors.buffer_bg))
            });
        let cancel_btn = Paragraph::new("Cancel")
            .alignment(ratatui::layout::Alignment::Center)
            .block(if self.action == ExportAction::Cancel {
                Block::bordered().border_style(Style::new().fg(colors.selected_cell_style_fg))
            } else {
                Block::bordered().border_style(Style::new().fg(colors.buffer_bg))
            });

        frame.render_widget(export_btn, buttons[0]);
        frame.render_widget(cancel_btn, buttons[1]);
    }
}
