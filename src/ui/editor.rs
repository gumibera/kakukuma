use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::app::App;
use crate::cell::{BlockChar, Cell, Color256};
use crate::input::CanvasArea;
use crate::tools::{self, ToolState};

/// Return the visual background color for an empty/transparent cell position.
fn grid_bg(x: usize, y: usize, show_grid: bool) -> Color {
    if show_grid {
        if (x + y).is_multiple_of(2) {
            Color::Indexed(235)
        } else {
            Color::Indexed(234)
        }
    } else {
        Color::Reset
    }
}

/// Resolve a half-block cell into (char, fg, bg) for terminal rendering,
/// treating Color256::BLACK halves as transparent (grid/background).
fn resolve_half_block(cell: Cell, x: usize, y: usize, show_grid: bool) -> (char, Color, Color) {
    let canvas_bg = Color256::BLACK;

    match cell.block {
        BlockChar::UpperHalf => {
            let top = cell.fg;
            let bottom = cell.bg;
            match (top == canvas_bg, bottom == canvas_bg) {
                (true, true) => (' ', Color::Reset, grid_bg(x, y, show_grid)),
                (false, true) => ('▀', top.to_ratatui(), grid_bg(x, y, show_grid)),
                (true, false) => ('▄', bottom.to_ratatui(), grid_bg(x, y, show_grid)),
                (false, false) => ('▀', top.to_ratatui(), bottom.to_ratatui()),
            }
        }
        BlockChar::LeftHalf => {
            let left = cell.fg;
            let right = cell.bg;
            match (left == canvas_bg, right == canvas_bg) {
                (true, true) => (' ', Color::Reset, grid_bg(x, y, show_grid)),
                (false, true) => ('▌', left.to_ratatui(), grid_bg(x, y, show_grid)),
                (true, false) => ('▐', right.to_ratatui(), grid_bg(x, y, show_grid)),
                (false, false) => ('▌', left.to_ratatui(), right.to_ratatui()),
            }
        }
        // LowerHalf and RightHalf are non-canonical but handle defensively
        BlockChar::LowerHalf => {
            let bottom = cell.fg;
            let top = cell.bg;
            match (top == canvas_bg, bottom == canvas_bg) {
                (true, true) => (' ', Color::Reset, grid_bg(x, y, show_grid)),
                (false, true) => ('▀', top.to_ratatui(), grid_bg(x, y, show_grid)),
                (true, false) => ('▄', bottom.to_ratatui(), grid_bg(x, y, show_grid)),
                (false, false) => ('▄', bottom.to_ratatui(), top.to_ratatui()),
            }
        }
        BlockChar::RightHalf => {
            let right = cell.fg;
            let left = cell.bg;
            match (left == canvas_bg, right == canvas_bg) {
                (true, true) => (' ', Color::Reset, grid_bg(x, y, show_grid)),
                (false, true) => ('▌', left.to_ratatui(), grid_bg(x, y, show_grid)),
                (true, false) => ('▐', right.to_ratatui(), grid_bg(x, y, show_grid)),
                (false, false) => ('▐', right.to_ratatui(), left.to_ratatui()),
            }
        }
        _ => unreachable!("resolve_half_block called for non-half-block"),
    }
}

/// Render the canvas editor and return the screen area for mouse mapping.
pub fn render(f: &mut Frame, app: &App, area: Rect) -> CanvasArea {
    let canvas_w = (app.canvas.width * 2) as u16;
    let canvas_h = app.canvas.height as u16;

    // Center the canvas in the available area
    let offset_x = (area.width.saturating_sub(canvas_w)) / 2;
    let offset_y = (area.height.saturating_sub(canvas_h)) / 2;

    let canvas_rect = Rect::new(
        area.x + offset_x,
        area.y + offset_y,
        canvas_w.min(area.width),
        canvas_h.min(area.height),
    );

    let widget = CanvasWidget { app };
    f.render_widget(widget, canvas_rect);

    CanvasArea {
        left: canvas_rect.x,
        top: canvas_rect.y,
        width: canvas_rect.width,
        height: canvas_rect.height,
    }
}

/// Render the canvas as a true terminal preview (1 char per cell, actual output size).
pub fn render_preview(f: &mut Frame, app: &App, area: Rect) {
    let canvas_w = app.canvas.width as u16;
    let canvas_h = app.canvas.height as u16;

    let offset_x = (area.width.saturating_sub(canvas_w)) / 2;
    let offset_y = (area.height.saturating_sub(canvas_h)) / 2;

    let canvas_rect = Rect::new(
        area.x + offset_x,
        area.y + offset_y,
        canvas_w.min(area.width),
        canvas_h.min(area.height),
    );

    let widget = TruePreviewWidget { app };
    f.render_widget(widget, canvas_rect);
}

struct CanvasWidget<'a> {
    app: &'a App,
}

impl<'a> CanvasWidget<'a> {
    fn is_in_tool_preview(&self, x: usize, y: usize) -> bool {
        let cursor = match self.app.cursor {
            Some(c) => c,
            None => return false,
        };
        match &self.app.tool_state {
            ToolState::LineStart { x: x0, y: y0 } => {
                let points = tools::bresenham_line(*x0, *y0, cursor.0, cursor.1);
                points.contains(&(x, y))
            }
            ToolState::RectStart { x: x0, y: y0 } => {
                let min_x = (*x0).min(cursor.0);
                let max_x = (*x0).max(cursor.0);
                let min_y = (*y0).min(cursor.1);
                let max_y = (*y0).max(cursor.1);
                let is_border = x == min_x || x == max_x || y == min_y || y == max_y;
                x >= min_x && x <= max_x && y >= min_y && y <= max_y && is_border
            }
            ToolState::Idle => false,
        }
    }
}

impl<'a> Widget for CanvasWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cw = self.app.canvas.width;
        let ch = self.app.canvas.height;
        for y in 0..ch {
            for x in 0..cw {
                let screen_x = area.x + (x as u16) * 2;
                let screen_y = area.y + y as u16;

                if screen_x + 1 >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                if let Some(cell) = self.app.canvas.get(x, y) {
                    let is_cursor = self.app.cursor == Some((x, y));

                    // Tool preview overlay (line/rect in progress)
                    let render_cell = if self.is_in_tool_preview(x, y) && !is_cursor {
                        tools::compose_cell(
                            cell,
                            self.app.active_block,
                            self.app.color,
                            Color256::BLACK,
                        )
                    } else {
                        cell
                    };

                    // Symmetry axis check
                    let on_h_axis = self.app.symmetry.has_horizontal()
                        && (x == cw / 2 - 1 || x == cw / 2);
                    let on_v_axis = self.app.symmetry.has_vertical()
                        && (y == ch / 2 - 1 || y == ch / 2);
                    let on_axis = (on_h_axis || on_v_axis) && !is_cursor;

                    // Horizontal half-blocks (▌▐): render each terminal char
                    // separately so the left/right split is clearly visible.
                    let is_horiz_half = matches!(
                        render_cell.block,
                        BlockChar::LeftHalf | BlockChar::RightHalf
                    );

                    if is_horiz_half && !is_cursor {
                        let (left, right) = match render_cell.block {
                            BlockChar::LeftHalf => (render_cell.fg, render_cell.bg),
                            _ => (render_cell.bg, render_cell.fg),
                        };
                        let canvas_bg = Color256::BLACK;
                        let left_bg = if left == canvas_bg {
                            grid_bg(x, y, true)
                        } else {
                            left.to_ratatui()
                        };
                        let right_bg = if right == canvas_bg {
                            grid_bg(x, y, true)
                        } else {
                            right.to_ratatui()
                        };
                        buf.set_string(screen_x, screen_y, " ", Style::default().bg(left_bg));
                        buf.set_string(screen_x + 1, screen_y, " ", Style::default().bg(right_bg));
                    } else {
                        let (ch, mut fg, mut bg) = match render_cell.block {
                            BlockChar::Full => {
                                let c = render_cell.fg.to_ratatui();
                                (render_cell.block.to_char(), c, c)
                            }
                            BlockChar::Empty => {
                                (' ', render_cell.fg.to_ratatui(), grid_bg(x, y, true))
                            }
                            _ => {
                                // Vertical half-blocks, or horizontal under cursor
                                resolve_half_block(render_cell, x, y, true)
                            }
                        };

                        if on_axis && render_cell.block == BlockChar::Empty {
                            bg = Color::Indexed(238);
                        }

                        if is_cursor {
                            std::mem::swap(&mut fg, &mut bg);
                        }

                        let style = Style::default().fg(fg).bg(bg);
                        let ch_str: String = std::iter::repeat_n(ch, 2).collect();
                        buf.set_string(screen_x, screen_y, &ch_str, style);
                    }
                }
            }
        }
    }
}

/// True terminal preview: 1 character per cell (actual output size).
struct TruePreviewWidget<'a> {
    app: &'a App,
}

impl<'a> Widget for TruePreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in 0..self.app.canvas.height {
            for x in 0..self.app.canvas.width {
                let screen_x = area.x + x as u16;
                let screen_y = area.y + y as u16;

                if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                if let Some(cell) = self.app.canvas.get(x, y) {
                    let (ch, fg, bg) = match cell.block {
                        BlockChar::Full => {
                            let c = cell.fg.to_ratatui();
                            (cell.block.to_char(), c, c)
                        }
                        BlockChar::Empty => {
                            (' ', cell.fg.to_ratatui(), Color::Reset)
                        }
                        BlockChar::UpperHalf | BlockChar::LowerHalf
                        | BlockChar::LeftHalf | BlockChar::RightHalf => {
                            resolve_half_block(cell, x, y, false)
                        }
                    };
                    let style = Style::default().fg(fg).bg(bg);
                    buf.set_string(screen_x, screen_y, ch.to_string(), style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- grid_bg tests ---

    #[test]
    fn grid_bg_even_cell_with_grid() {
        assert_eq!(grid_bg(0, 0, true), Color::Indexed(235));
        assert_eq!(grid_bg(2, 4, true), Color::Indexed(235));
    }

    #[test]
    fn grid_bg_odd_cell_with_grid() {
        assert_eq!(grid_bg(1, 0, true), Color::Indexed(234));
        assert_eq!(grid_bg(0, 1, true), Color::Indexed(234));
    }

    #[test]
    fn grid_bg_without_grid() {
        assert_eq!(grid_bg(0, 0, false), Color::Reset);
        assert_eq!(grid_bg(1, 0, false), Color::Reset);
    }

    // --- resolve_half_block tests ---

    fn cell(block: BlockChar, fg: u8, bg: u8) -> Cell {
        Cell { block, fg: Color256(fg), bg: Color256(bg) }
    }

    #[test]
    fn upper_half_one_transparent_bottom() {
        // UpperHalf(red, black) → red top, grid bottom
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::UpperHalf, 1, 0), 0, 0, true);
        assert_eq!(ch, '▀');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(235));
    }

    #[test]
    fn upper_half_both_opaque() {
        // UpperHalf(red, blue) → no transparency
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::UpperHalf, 1, 4), 0, 0, true);
        assert_eq!(ch, '▀');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(4));
    }

    #[test]
    fn upper_half_one_transparent_top_flips() {
        // UpperHalf(black, blue) → flips to ▄ blue on grid
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::UpperHalf, 0, 4), 0, 0, true);
        assert_eq!(ch, '▄');
        assert_eq!(fg, Color::Indexed(4));
        assert_eq!(bg, Color::Indexed(235));
    }

    #[test]
    fn upper_half_both_transparent() {
        // UpperHalf(black, black) → empty
        let (ch, _fg, bg) = resolve_half_block(cell(BlockChar::UpperHalf, 0, 0), 0, 0, true);
        assert_eq!(ch, ' ');
        assert_eq!(bg, Color::Indexed(235));
    }

    #[test]
    fn left_half_one_transparent_right() {
        // LeftHalf(red, black) → red left, grid right
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::LeftHalf, 1, 0), 1, 0, true);
        assert_eq!(ch, '▌');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(234));
    }

    #[test]
    fn left_half_flips_when_left_transparent() {
        // LeftHalf(black, red) → flips to ▐ red on grid
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::LeftHalf, 0, 1), 0, 0, true);
        assert_eq!(ch, '▐');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(235));
    }

    #[test]
    fn lower_half_defensive() {
        // LowerHalf(blue, black) → fg=bottom=blue, bg=top=black
        // top transparent, bottom opaque → ▄ blue on grid
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::LowerHalf, 4, 0), 0, 0, true);
        assert_eq!(ch, '▄');
        assert_eq!(fg, Color::Indexed(4));
        assert_eq!(bg, Color::Indexed(235));
    }

    #[test]
    fn right_half_defensive() {
        // RightHalf(red, black) → fg=right=red, bg=left=black
        // left transparent, right opaque → ▐ red on grid
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::RightHalf, 1, 0), 0, 0, true);
        assert_eq!(ch, '▐');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(235));
    }

    #[test]
    fn resolve_grid_off_uses_reset() {
        // With grid off, transparent halves use Color::Reset
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::UpperHalf, 1, 0), 0, 0, false);
        assert_eq!(ch, '▀');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Reset);
    }

    #[test]
    fn left_half_both_opaque() {
        // LeftHalf(red, blue) → no transparency
        let (ch, fg, bg) = resolve_half_block(cell(BlockChar::LeftHalf, 1, 4), 0, 0, true);
        assert_eq!(ch, '▌');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(4));
    }
}
