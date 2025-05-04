use ratatui::layout::{Constraint, Flex, Layout, Rect};

pub fn popup_area(area: Rect, percent_x: u32, percent_y: u32) -> Rect {
    let vertical = Layout::vertical([Constraint::Ratio(percent_y, 9)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Ratio(percent_x, 9)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
