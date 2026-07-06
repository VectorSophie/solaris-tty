//! Off-screen cell grid with a 1/z depth buffer, a braille sub-pixel layer for
//! trails, and dirty-cell diffing. Ported/streamlined from checkmate-tty.

use crossterm::style::Color;

use super::cell::Cell;

pub struct FrameBuffer {
    pub width: u16,
    pub height: u16,
    current: Vec<Cell>,
    previous: Vec<Cell>,
    // Half-block pixel layer at width × 2·height: bodies and stars render here
    // as square-ish colour pixels, then composite into ▀/▄ cells. Gives smooth
    // shaded spheres instead of ASCII-ramp text.
    px_color: Vec<Color>,
    px_depth: Vec<f32>,
    // Braille trail layer: one accumulating 8-dot mask per cell, with the
    // nearest depth and its colour. Composited over bodies.
    braille_mask: Vec<u8>,
    braille_depth: Vec<f32>,
    braille_color: Vec<Color>,
}

impl FrameBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let n = width as usize * height as usize;
        let pn = n * 2;
        Self {
            width,
            height,
            current: vec![Cell::EMPTY; n],
            previous: vec![Cell { ch: '\x01', ..Cell::EMPTY }; n],
            px_color: vec![Color::Reset; pn],
            px_depth: vec![0.0; pn],
            braille_mask: vec![0; n],
            braille_depth: vec![0.0; n],
            braille_color: vec![Color::Reset; n],
        }
    }

    /// Pixel-space height for body/star rendering (2× cell height).
    #[inline]
    pub fn pixel_height(&self) -> u16 {
        self.height * 2
    }

    /// Depth-tested write into the half-block pixel layer. `px` in [0,W),
    /// `py` in [0,2·H).
    pub fn write_pixel(&mut self, px: i32, py: i32, color: Color, depth: f32) {
        if px < 0 || py < 0 || px as u16 >= self.width || py as u16 >= self.pixel_height() {
            return;
        }
        let idx = py as usize * self.width as usize + px as usize;
        if depth >= self.px_depth[idx] {
            self.px_depth[idx] = depth;
            self.px_color[idx] = color;
        }
    }

    /// Composite the pixel layer into cells as ▀/▄ half-blocks (top pixel = fg,
    /// bottom = bg). Depth-tested against existing cells.
    pub fn composite_pixels(&mut self) {
        let w = self.width as usize;
        for cy in 0..self.height as usize {
            for cx in 0..w {
                let top = cy * 2 * w + cx;
                let bot = (cy * 2 + 1) * w + cx;
                let (td, bd) = (self.px_depth[top], self.px_depth[bot]);
                if td <= 0.0 && bd <= 0.0 {
                    continue;
                }
                let (ch, fg, bg, depth) = if td > 0.0 && bd > 0.0 {
                    ('▀', self.px_color[top], self.px_color[bot], td.max(bd))
                } else if td > 0.0 {
                    ('▀', self.px_color[top], Color::Reset, td)
                } else {
                    ('▄', self.px_color[bot], Color::Reset, bd)
                };
                let ci = cy * w + cx;
                if depth >= self.current[ci].depth {
                    self.current[ci] = Cell { ch, fg, bg, depth };
                }
            }
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        *self = Self::new(width, height);
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    #[inline]
    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u16) < self.width && (y as u16) < self.height
    }

    /// Depth-tested cell write (for body discs). Returns true if it won.
    pub fn write_depth(&mut self, x: u16, y: u16, cell: Cell) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = self.idx(x, y);
        if cell.depth >= self.current[idx].depth {
            self.current[idx] = cell;
            true
        } else {
            false
        }
    }

    /// Overlay write, ignores depth (UI panels, labels).
    pub fn write_overlay(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            let idx = self.idx(x, y);
            self.current[idx] = cell;
        }
    }

    pub fn write_str(&mut self, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
        for (i, ch) in s.chars().enumerate() {
            let cx = x as usize + i;
            if cx >= self.width as usize || y >= self.height {
                break;
            }
            self.write_overlay(cx as u16, y, Cell { ch, fg, bg, depth: f32::MAX });
        }
    }

    /// Plot a braille sub-pixel. `sx`/`sy` are sub-pixel coords: sx in [0,2·W),
    /// sy in [0,4·H). Accumulates dots per cell, keeping the nearest depth.
    pub fn plot_braille(&mut self, sx: i32, sy: i32, depth: f32, color: Color) {
        if sx < 0 || sy < 0 {
            return;
        }
        let (cx, cy) = (sx as u16 / 2, sy as u16 / 4);
        if cx >= self.width || cy >= self.height {
            return;
        }
        let idx = self.idx(cx, cy);
        let (dx, dy) = ((sx as u16 % 2) as u8, (sy as u16 % 4) as u8);
        self.braille_mask[idx] |= braille_bit(dx, dy);
        if depth > self.braille_depth[idx] {
            self.braille_depth[idx] = depth;
            self.braille_color[idx] = color;
        }
    }

    /// Composite the braille trail layer into cells, depth-tested against the
    /// body discs already written. Call after bodies, before UI.
    pub fn composite_braille(&mut self) {
        for idx in 0..self.current.len() {
            let mask = self.braille_mask[idx];
            if mask == 0 {
                continue;
            }
            let depth = self.braille_depth[idx];
            if depth >= self.current[idx].depth {
                let ch = char::from_u32(0x2800 + mask as u32).unwrap_or('.');
                self.current[idx] = Cell {
                    ch,
                    fg: self.braille_color[idx],
                    bg: Color::Reset,
                    depth,
                };
            }
        }
    }

    pub fn clear(&mut self) {
        for c in &mut self.current {
            *c = Cell::EMPTY;
        }
        for d in &mut self.px_depth {
            *d = 0.0;
        }
        for m in &mut self.braille_mask {
            *m = 0;
        }
        for d in &mut self.braille_depth {
            *d = 0.0;
        }
    }

    pub fn dirty_iter(&self) -> impl Iterator<Item = (u16, u16, &Cell)> {
        let width = self.width as usize;
        self.current.iter().enumerate().filter_map(move |(idx, cell)| {
            if cell != &self.previous[idx] {
                Some(((idx % width) as u16, (idx / width) as u16, cell))
            } else {
                None
            }
        })
    }

    pub fn swap(&mut self) {
        self.previous.copy_from_slice(&self.current);
    }

    /// Dump the current cells as a plain-text grid (chars only, no color).
    /// For headless verification without a real terminal.
    pub fn to_text(&self) -> String {
        let mut s = String::with_capacity((self.width as usize + 1) * self.height as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                s.push(self.current[self.idx(x, y)].ch);
            }
            s.push('\n');
        }
        s
    }
}

/// Unicode braille dot layout within a cell (2 wide × 4 tall):
///   (0,0)=0x01 (1,0)=0x08
///   (0,1)=0x02 (1,1)=0x10
///   (0,2)=0x04 (1,2)=0x20
///   (0,3)=0x40 (1,3)=0x80
fn braille_bit(dx: u8, dy: u8) -> u8 {
    match (dx, dy) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    }
}
