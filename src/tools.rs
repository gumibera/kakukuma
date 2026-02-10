use crate::canvas::Canvas;
use crate::cell::{BlockChar, Cell, Color256};
use crate::history::CellMutation;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolKind {
    Pencil,
    Eraser,
    Line,
    Rectangle,
    Fill,
    Eyedropper,
}

impl ToolKind {
    pub fn name(self) -> &'static str {
        match self {
            ToolKind::Pencil => "Pencil",
            ToolKind::Eraser => "Eraser",
            ToolKind::Line => "Line",
            ToolKind::Rectangle => "Rect",
            ToolKind::Fill => "Fill",
            ToolKind::Eyedropper => "Pick",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ToolKind::Pencil => "\u{270F}",    // ✏
            ToolKind::Eraser => "\u{25FB}",    // ◻
            ToolKind::Line => "\u{2571}",      // ╱
            ToolKind::Rectangle => "\u{25AD}", // ▭
            ToolKind::Fill => "\u{25C9}",      // ◉
            ToolKind::Eyedropper => "\u{25C8}", // ◈
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            ToolKind::Pencil => "P",
            ToolKind::Eraser => "E",
            ToolKind::Line => "L",
            ToolKind::Rectangle => "R",
            ToolKind::Fill => "F",
            ToolKind::Eyedropper => "I",
        }
    }

    pub const ALL: [ToolKind; 6] = [
        ToolKind::Pencil,
        ToolKind::Eraser,
        ToolKind::Line,
        ToolKind::Rectangle,
        ToolKind::Fill,
        ToolKind::Eyedropper,
    ];
}

#[derive(Clone, Debug)]
pub enum ToolState {
    Idle,
    LineStart { x: usize, y: usize },
    RectStart { x: usize, y: usize },
}

/// Place a single cell (pencil).
pub fn pencil(
    canvas: &Canvas,
    x: usize,
    y: usize,
    block: BlockChar,
    fg: Color256,
    bg: Color256,
) -> Vec<CellMutation> {
    if let Some(old) = canvas.get(x, y) {
        let new = Cell { block, fg, bg };
        if old != new {
            vec![CellMutation { x, y, old, new }]
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

/// Erase a cell (set to empty with default bg).
pub fn eraser(canvas: &Canvas, x: usize, y: usize) -> Vec<CellMutation> {
    if let Some(old) = canvas.get(x, y) {
        let new = Cell::default();
        if old != new {
            vec![CellMutation { x, y, old, new }]
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

/// Bresenham's line algorithm. Returns list of (x, y) points.
pub fn bresenham_line(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize)> {
    let mut points = Vec::new();
    let (mut x0, mut y0) = (x0 as isize, y0 as isize);
    let (x1, y1) = (x1 as isize, y1 as isize);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        points.push((x0 as usize, y0 as usize));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    points
}

/// Draw a line from (x0,y0) to (x1,y1).
#[allow(clippy::too_many_arguments)]
pub fn line(
    canvas: &Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    block: BlockChar,
    fg: Color256,
    bg: Color256,
) -> Vec<CellMutation> {
    let points = bresenham_line(x0, y0, x1, y1);
    let new = Cell { block, fg, bg };
    let mut mutations = Vec::new();
    for (x, y) in points {
        if let Some(old) = canvas.get(x, y) {
            if old != new {
                mutations.push(CellMutation { x, y, old, new });
            }
        }
    }
    mutations
}

/// Draw a rectangle outline from (x0,y0) to (x1,y1).
#[allow(clippy::too_many_arguments)]
pub fn rectangle(
    canvas: &Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    block: BlockChar,
    fg: Color256,
    bg: Color256,
    filled: bool,
) -> Vec<CellMutation> {
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);
    let new = Cell { block, fg, bg };
    let mut mutations = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let is_border = x == min_x || x == max_x || y == min_y || y == max_y;
            if filled || is_border {
                if let Some(old) = canvas.get(x, y) {
                    if old != new {
                        mutations.push(CellMutation { x, y, old, new });
                    }
                }
            }
        }
    }
    mutations
}

/// Iterative flood fill from (start_x, start_y).
pub fn flood_fill(
    canvas: &Canvas,
    start_x: usize,
    start_y: usize,
    block: BlockChar,
    fg: Color256,
    bg: Color256,
) -> Vec<CellMutation> {
    let target = match canvas.get(start_x, start_y) {
        Some(cell) => cell,
        None => return vec![],
    };

    let new = Cell { block, fg, bg };
    if target == new {
        return vec![]; // No-op: already the target color
    }

    let w = canvas.width;
    let h = canvas.height;
    let mut mutations = Vec::new();
    let mut visited = vec![false; w * h];
    let mut stack = vec![(start_x, start_y)];

    while let Some((x, y)) = stack.pop() {
        if x >= w || y >= h || visited[y * w + x] {
            continue;
        }
        if let Some(cell) = canvas.get(x, y) {
            if cell != target {
                continue;
            }
        } else {
            continue;
        }

        visited[y * w + x] = true;
        mutations.push(CellMutation {
            x,
            y,
            old: target,
            new,
        });

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }

    mutations
}

/// Pick color from a canvas cell.
pub fn eyedropper(canvas: &Canvas, x: usize, y: usize) -> Option<(Color256, Color256, BlockChar)> {
    canvas.get(x, y).map(|cell| (cell.fg, cell.bg, cell.block))
}

/// Compose a new cell from a drawing operation. All block types replace the
/// cell entirely — half-blocks stamp cleanly with the uncovered half transparent.
pub fn compose_cell(_existing: Cell, new_block: BlockChar, new_fg: Color256, new_bg: Color256) -> Cell {
    Cell { block: new_block, fg: new_fg, bg: new_bg }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bresenham_horizontal() {
        let points = bresenham_line(0, 0, 5, 0);
        assert_eq!(points.len(), 6);
        for (i, &(x, y)) in points.iter().enumerate() {
            assert_eq!(x, i);
            assert_eq!(y, 0);
        }
    }

    #[test]
    fn test_bresenham_vertical() {
        let points = bresenham_line(0, 0, 0, 5);
        assert_eq!(points.len(), 6);
        for (i, &(x, y)) in points.iter().enumerate() {
            assert_eq!(x, 0);
            assert_eq!(y, i);
        }
    }

    #[test]
    fn test_bresenham_diagonal() {
        let points = bresenham_line(0, 0, 5, 5);
        assert_eq!(points.len(), 6);
        for (i, &(x, y)) in points.iter().enumerate() {
            assert_eq!(x, i);
            assert_eq!(y, i);
        }
    }

    #[test]
    fn test_bresenham_single_point() {
        let points = bresenham_line(3, 3, 3, 3);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], (3, 3));
    }

    #[test]
    fn test_bresenham_steep() {
        let points = bresenham_line(0, 0, 2, 6);
        assert_eq!(points.first(), Some(&(0, 0)));
        assert_eq!(points.last(), Some(&(2, 6)));
        assert_eq!(points.len(), 7);
        for w in points.windows(2) {
            assert!(w[1].1 >= w[0].1);
        }
    }

    #[test]
    fn test_bresenham_shallow() {
        let points = bresenham_line(0, 0, 6, 2);
        assert_eq!(points.first(), Some(&(0, 0)));
        assert_eq!(points.last(), Some(&(6, 2)));
        assert_eq!(points.len(), 7);
        for w in points.windows(2) {
            assert!(w[1].0 >= w[0].0);
        }
    }

    #[test]
    fn test_bresenham_reverse() {
        let fwd = bresenham_line(0, 0, 5, 3);
        let rev = bresenham_line(5, 3, 0, 0);
        assert_eq!(fwd.len(), rev.len());
        for p in &fwd {
            assert!(rev.contains(p));
        }
    }

    #[test]
    fn test_rectangle_single_cell() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas, 5, 5, 5, 5,
            BlockChar::Full, Color256(1), Color256::BLACK, false,
        );
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].x, 5);
        assert_eq!(mutations[0].y, 5);
    }

    #[test]
    fn test_rectangle_wide() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas, 0, 0, 9, 0,
            BlockChar::Full, Color256(1), Color256::BLACK, false,
        );
        assert_eq!(mutations.len(), 10);
    }

    #[test]
    fn test_rectangle_tall() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas, 0, 0, 0, 7,
            BlockChar::Full, Color256(1), Color256::BLACK, false,
        );
        assert_eq!(mutations.len(), 8);
    }

    #[test]
    fn test_flood_fill_boundary() {
        let mut canvas = Canvas::new();
        let wall = Cell {
            block: BlockChar::Full,
            fg: Color256(1),
            bg: Color256::BLACK,
        };
        for x in 0..3 {
            canvas.set(x, 0, wall);
            canvas.set(x, 2, wall);
        }
        canvas.set(0, 1, wall);
        canvas.set(2, 1, wall);
        let mutations = flood_fill(&canvas, 1, 1, BlockChar::Full, Color256(4), Color256::BLACK);
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].x, 1);
        assert_eq!(mutations[0].y, 1);
    }

    #[test]
    fn test_flood_fill_noop() {
        let canvas = Canvas::new();
        let mutations = flood_fill(
            &canvas,
            0,
            0,
            BlockChar::Empty,
            Color256::WHITE,
            Color256::BLACK,
        );
        assert_eq!(mutations.len(), 0);
    }

    #[test]
    fn test_flood_fill_entire_canvas() {
        let canvas = Canvas::new();
        let mutations = flood_fill(
            &canvas,
            0,
            0,
            BlockChar::Full,
            Color256(1),
            Color256::BLACK,
        );
        assert_eq!(mutations.len(), 32 * 32);
    }

    #[test]
    fn test_rectangle_outline() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas,
            0,
            0,
            3,
            3,
            BlockChar::Full,
            Color256(1),
            Color256::BLACK,
            false,
        );
        assert_eq!(mutations.len(), 12);
    }

    #[test]
    fn test_rectangle_filled() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas,
            0,
            0,
            3,
            3,
            BlockChar::Full,
            Color256(1),
            Color256::BLACK,
            true,
        );
        assert_eq!(mutations.len(), 16);
    }

    // --- compose_cell tests ---

    const RED: Color256 = Color256(1);
    const BLUE: Color256 = Color256(4);
    const GREEN: Color256 = Color256(2);

    fn empty_cell() -> Cell {
        Cell::default() // Empty, fg=White, bg=Black
    }

    #[test]
    fn compose_full_block_replaces_entirely() {
        let existing = Cell { block: BlockChar::UpperHalf, fg: RED, bg: BLUE };
        let result = compose_cell(existing, BlockChar::Full, GREEN, Color256::BLACK);
        assert_eq!(result, Cell { block: BlockChar::Full, fg: GREEN, bg: Color256::BLACK });
    }

    #[test]
    fn compose_empty_block_replaces_entirely() {
        let existing = Cell { block: BlockChar::Full, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::Empty, Color256::WHITE, Color256::BLACK);
        assert_eq!(result, Cell { block: BlockChar::Empty, fg: Color256::WHITE, bg: Color256::BLACK });
    }

    #[test]
    fn compose_upper_half_on_empty() {
        let result = compose_cell(empty_cell(), BlockChar::UpperHalf, RED, Color256::BLACK);
        // Top=RED, Bottom=BLACK (from empty bg) → UpperHalf(RED, BLACK)
        assert_eq!(result.block, BlockChar::UpperHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_lower_half_on_empty() {
        let result = compose_cell(empty_cell(), BlockChar::LowerHalf, RED, Color256::BLACK);
        // Replace: LowerHalf(RED, BLACK)
        assert_eq!(result.block, BlockChar::LowerHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_upper_red_then_lower_blue() {
        // Existing: UpperHalf red — ignored, replace with LowerHalf blue
        let existing = Cell { block: BlockChar::UpperHalf, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::LowerHalf, BLUE, Color256::BLACK);
        // Replace: LowerHalf(BLUE, BLACK)
        assert_eq!(result.block, BlockChar::LowerHalf);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_lower_blue_then_upper_red() {
        // Existing: LowerHalf blue — ignored, replace with UpperHalf red
        let existing = Cell { block: BlockChar::LowerHalf, fg: BLUE, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::UpperHalf, RED, Color256::BLACK);
        // Replace: UpperHalf(RED, BLACK)
        assert_eq!(result.block, BlockChar::UpperHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_lower_half_replaces_regardless_of_existing() {
        let existing = Cell { block: BlockChar::UpperHalf, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::LowerHalf, RED, Color256::BLACK);
        // Replace: LowerHalf(RED, BLACK) — no collapse to Full
        assert_eq!(result.block, BlockChar::LowerHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_left_half_on_empty() {
        let result = compose_cell(empty_cell(), BlockChar::LeftHalf, RED, Color256::BLACK);
        // Left=RED, Right=BLACK → LeftHalf(RED, BLACK)
        assert_eq!(result.block, BlockChar::LeftHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_right_half_on_empty() {
        let result = compose_cell(empty_cell(), BlockChar::RightHalf, RED, Color256::BLACK);
        // Replace: RightHalf(RED, BLACK)
        assert_eq!(result.block, BlockChar::RightHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_left_then_right_horizontal() {
        // Existing: LeftHalf red — ignored, replace with RightHalf blue
        let existing = Cell { block: BlockChar::LeftHalf, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::RightHalf, BLUE, Color256::BLACK);
        // Replace: RightHalf(BLUE, BLACK)
        assert_eq!(result.block, BlockChar::RightHalf);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_right_half_replaces_regardless_of_existing() {
        let existing = Cell { block: BlockChar::LeftHalf, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::RightHalf, RED, Color256::BLACK);
        // Replace: RightHalf(RED, BLACK) — no collapse to Full
        assert_eq!(result.block, BlockChar::RightHalf);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_cross_axis_replaces_entirely() {
        // Existing: LeftHalf(RED, BLACK) — ignored, replace with UpperHalf blue
        let existing = Cell { block: BlockChar::LeftHalf, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::UpperHalf, BLUE, Color256::BLACK);
        // Replace: UpperHalf(BLUE, BLACK)
        assert_eq!(result.block, BlockChar::UpperHalf);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_half_on_full_replaces_entirely() {
        let existing = Cell { block: BlockChar::Full, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::UpperHalf, BLUE, Color256::BLACK);
        // Replace: UpperHalf(BLUE, BLACK) — no color bleed from existing
        assert_eq!(result.block, BlockChar::UpperHalf);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, Color256::BLACK);
    }

    #[test]
    fn compose_idempotent_same_half_same_color() {
        let existing = Cell { block: BlockChar::UpperHalf, fg: RED, bg: Color256::BLACK };
        let result = compose_cell(existing, BlockChar::UpperHalf, RED, Color256::BLACK);
        // Replace: same result — still idempotent
        assert_eq!(result, existing);
    }
}
