use crate::model::PortInfo;
use crate::ui::theme::TableColors;
use crate::util::popup_area;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    prelude::Style,
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KillAction {
    Kill,
    Cancel,
}

impl Default for KillAction {
    fn default() -> Self {
        KillAction::Kill
    }
}

/// A popup component that asks “Kill process?” and lets you choose Kill/Cancel.

#[derive(Debug)]
pub struct KillComponent {
    /// whether popup is visible
    pub display: bool,
    /// which process we’re about to kill
    pub item: Option<PortInfo>,
    /// which button is focused
    pub action: KillAction,
}

impl Default for KillComponent {
    fn default() -> Self {
        Self {
            display: false,
            item: None,
            action: KillAction::Kill,
        }
    }
}

impl KillComponent {
    /// Show the popup for this `PortInfo`
    pub fn show(&mut self, item: PortInfo) {
        self.display = true;
        self.item = Some(item);
        self.action = KillAction::Kill;
    }

    /// Hide the popup (Cancel)
    pub fn hide(&mut self) {
        self.display = false;
        self.item = None;
    }

    /// Move focus left (towards Kill)
    pub fn focus_kill(&mut self) {
        self.action = KillAction::Kill;
    }

    /// Move focus right (towards Cancel)
    pub fn focus_cancel(&mut self) {
        self.action = KillAction::Cancel;
    }

    /// Returns true if user pressed Enter on “Kill”
    pub fn confirm(&mut self) -> bool {
        let do_kill = self.action == KillAction::Kill;
        self.hide();
        do_kill
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
            .title("Kill");

        let area = popup_area(area, 4, 5);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        // split into prompt / description / buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);

        // 1) prompt line
        let prompt = match &self.item {
            Some(item) => {
                let t = format!(
                    "Kill {} {:?} port {} ?",
                    item.process_name, item.port_state, item.port
                );
                Paragraph::new(Line::from(t))
            }
            None => Paragraph::new(Line::from("Kill ?")),
        }
        .style(Style::default().fg(colors.row_fg).bg(colors.buffer_bg))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
        frame.render_widget(
            prompt,
            chunks[1].inner(Margin {
                horizontal: 2,
                vertical: 0,
            }),
        );

        // 2) description
        let desc = match &self.item {
            Some(item) => {
                let s = format!(
                    "Ending this process may disrupt services using port {}. Proceeding could result in data loss, network issues, or instability.",
                    item.port
                );
                Paragraph::new(Line::from(s))
            }
            None => Paragraph::new(Line::from("Kill ?")),
        }
            .style(Style::default().fg(colors.row_fg).bg(colors.buffer_bg))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(
            desc,
            chunks[2].inner(Margin {
                horizontal: 2,
                vertical: 0,
            }),
        );

        // 3) buttons
        let btns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
            .flex(Flex::Center)
            .split(chunks[4]);

        let kill_btn = Paragraph::new("Kill")
            .alignment(ratatui::layout::Alignment::Center)
            .block(if self.action == KillAction::Kill {
                Block::bordered().border_style(Style::new().fg(colors.selected_cell_style_fg))
            } else {
                Block::bordered()
            });
        let cancel_btn = Paragraph::new("Cancel")
            .alignment(ratatui::layout::Alignment::Center)
            .block(if self.action == KillAction::Cancel {
                Block::bordered().border_style(Style::new().fg(colors.selected_cell_style_fg))
            } else {
                Block::bordered()
            });

        frame.render_widget(kill_btn, btns[0]);
        frame.render_widget(cancel_btn, btns[1]);
    }
}
