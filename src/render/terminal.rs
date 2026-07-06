//! Terminal setup/teardown and framebuffer flush. Ported from checkmate-tty.

use std::io::{stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, queue,
    style::{self, Color},
    terminal::{self, ClearType},
};

use super::framebuffer::FrameBuffer;

/// Restore the terminal before a panic prints, so a crash never leaves the
/// user in raw mode on the alternate screen.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        default_hook(info);
    }));
}

pub fn setup() -> Result<()> {
    terminal::enable_raw_mode()?;
    execute!(
        stdout(),
        terminal::EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide,
        terminal::Clear(ClearType::All),
    )?;
    Ok(())
}

pub fn restore() -> Result<()> {
    let _ = execute!(
        stdout(),
        cursor::Show,
        DisableMouseCapture,
        style::ResetColor,
        terminal::LeaveAlternateScreen,
    );
    let _ = terminal::disable_raw_mode();
    Ok(())
}

pub fn size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}

/// Write only dirty cells, wrapped in a synchronized update = one flush/frame.
pub fn flush(fb: &FrameBuffer) -> Result<()> {
    let mut out = stdout().lock();
    queue!(out, terminal::BeginSynchronizedUpdate)?;

    let mut last_fg = Color::Reset;
    let mut last_bg = Color::Reset;
    for (x, y, cell) in fb.dirty_iter() {
        queue!(out, cursor::MoveTo(x, y))?;
        if cell.fg != last_fg {
            queue!(out, style::SetForegroundColor(cell.fg))?;
            last_fg = cell.fg;
        }
        if cell.bg != last_bg {
            queue!(out, style::SetBackgroundColor(cell.bg))?;
            last_bg = cell.bg;
        }
        queue!(out, style::Print(cell.ch))?;
    }

    queue!(out, terminal::EndSynchronizedUpdate)?;
    out.flush()?;
    Ok(())
}
