use crate::ui::keybindings_component::Keybinding;
use unicode_width::UnicodeWidthStr;

pub fn keybindings_constraint_len_calculator(items: &[Keybinding]) -> (u16, u16) {
    let combo = items
        .iter()
        .map(Keybinding::combo)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    let description = items
        .iter()
        .map(Keybinding::description)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (combo as u16, description as u16)
}
