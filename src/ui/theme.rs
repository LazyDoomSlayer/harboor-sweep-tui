use ratatui::prelude::Color;
use ratatui::style::palette::tailwind;

pub const PALETTES: [tailwind::Palette; 5] = [
    tailwind::GRAY,
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

#[derive(Debug)]
pub struct Theme {
    /// index into PALETTES
    pub idx: usize,
    pub table: TableColors,
}

impl Default for Theme {
    fn default() -> Self {
        let idx = 0;
        Theme {
            idx,
            table: TableColors::new(&PALETTES[idx]),
        }
    }
}

impl Theme {
    pub fn cycle_next(&mut self) {
        self.idx = (self.idx + 1) % PALETTES.len();
        self.table = TableColors::new(&PALETTES[self.idx]);
    }
    pub fn cycle_prev(&mut self) {
        let len = PALETTES.len();
        self.idx = (self.idx + len - 1) % len;
        self.table = TableColors::new(&PALETTES[self.idx]);
    }
}

/// exactly your old TableColors, moved here
#[derive(Clone, Debug)]
pub struct TableColors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub selected_row_style_fg: Color,
    pub selected_cell_style_fg: Color,
    pub footer_border_color: Color,
}

impl TableColors {
    pub const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            footer_border_color: color.c400,
        }
    }
}
