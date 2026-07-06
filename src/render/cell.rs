use crossterm::style::Color;

/// One terminal character cell. `depth` is inverse depth (1/w): larger = closer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub depth: f32,
}

impl Cell {
    pub const EMPTY: Self = Self {
        ch: ' ',
        fg: Color::Reset,
        bg: Color::Reset,
        depth: 0.0,
    };
}
