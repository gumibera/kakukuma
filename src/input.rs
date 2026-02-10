use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, AppMode};
use crate::canvas::Canvas;
use crate::cell::Color256;
use crate::history::History;
use crate::palette::{PaletteItem, PaletteSection};
use crate::tools::{ToolKind, ToolState};

/// Canvas area position in terminal coordinates.
/// Set by the UI renderer each frame.
pub struct CanvasArea {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
}

impl CanvasArea {
    /// Convert screen coordinates to canvas cell coordinates.
    /// Returns None if outside canvas bounds.
    pub fn screen_to_canvas(&self, screen_x: u16, screen_y: u16) -> Option<(usize, usize)> {
        if screen_x < self.left || screen_y < self.top {
            return None;
        }
        let rel_x = screen_x - self.left;
        let rel_y = screen_y - self.top;
        if rel_x >= self.width || rel_y >= self.height {
            return None;
        }
        // Each canvas cell is 2 chars wide
        let canvas_x = (rel_x / 2) as usize;
        let canvas_y = rel_y as usize;
        Some((canvas_x, canvas_y))
    }
}

pub fn handle_event(app: &mut App, event: Event, canvas_area: &CanvasArea) {
    match app.mode {
        AppMode::Help => {
            // Any key dismisses help
            if matches!(event, Event::Key(_)) {
                app.mode = AppMode::Normal;
            }
            return;
        }
        AppMode::Quitting => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.running = false;
                    }
                    _ => {
                        app.mode = AppMode::Normal;
                    }
                }
            }
            return;
        }
        AppMode::Recovery => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.recover_autosave();
                    }
                    _ => {
                        app.recovery_path = None;
                        app.mode = AppMode::Normal;
                    }
                }
            }
            return;
        }
        AppMode::FileDialog => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_file_dialog(app, code);
            }
            return;
        }
        AppMode::ExportDialog => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_export_dialog(app, code);
            }
            return;
        }
        AppMode::SaveAs => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::SaveAs);
            }
            return;
        }
        AppMode::ExportFile => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::ExportFile);
            }
            return;
        }
        AppMode::ColorSliders => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_color_sliders(app, code);
            }
            return;
        }
        AppMode::PaletteDialog => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_palette_dialog(app, code);
            }
            return;
        }
        AppMode::PaletteNameInput => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::PaletteName);
            }
            return;
        }
        AppMode::PaletteRename => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::PaletteRename);
            }
            return;
        }
        AppMode::PaletteExport => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::PaletteExport);
            }
            return;
        }
        AppMode::NewCanvas => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_new_canvas(app, code);
            }
            return;
        }
        _ => {}
    }

    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Mouse(mouse) => handle_mouse(app, mouse, canvas_area),
        Event::Resize(_, _) => {} // Layout handles this automatically
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl combinations
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('z') => {
                app.undo();
                return;
            }
            KeyCode::Char('y') => {
                app.redo();
                return;
            }
            KeyCode::Char('s') => {
                // Save
                if !app.save_project() {
                    // No path set — prompt for name
                    app.text_input = app
                        .project_name
                        .clone()
                        .unwrap_or_else(|| "untitled".to_string());
                    app.mode = AppMode::SaveAs;
                }
                return;
            }
            KeyCode::Char('o') => {
                // Open file dialog
                app.open_file_dialog();
                return;
            }
            KeyCode::Char('n') => {
                // New canvas dialog
                app.new_canvas_width = app.canvas.width;
                app.new_canvas_height = app.canvas.height;
                app.new_canvas_cursor = 0;
                app.mode = AppMode::NewCanvas;
                return;
            }
            KeyCode::Char('t') => {
                app.cycle_theme();
                return;
            }
            KeyCode::Char('e') => {
                // Export dialog
                app.export_format = 0;
                app.export_dest = 0;
                app.export_cursor = 0;
                app.mode = AppMode::ExportDialog;
                return;
            }
            KeyCode::Char('c') => {
                if app.dirty {
                    app.mode = AppMode::Quitting;
                    app.set_status("Unsaved changes. Quit? (y/n)");
                } else {
                    app.running = false;
                }
                return;
            }
            _ => return,
        }
    }

    match key.code {
        // Tool selection
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.active_tool = ToolKind::Pencil;
            app.cancel_tool();
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            app.active_tool = ToolKind::Eraser;
            app.cancel_tool();
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.active_tool = ToolKind::Line;
            app.cancel_tool();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.active_tool = ToolKind::Rectangle;
            app.cancel_tool();
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            app.active_tool = ToolKind::Fill;
            app.cancel_tool();
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            app.active_tool = ToolKind::Eyedropper;
            app.cancel_tool();
        }

        // Symmetry
        KeyCode::Char('h') | KeyCode::Char('H') => {
            app.symmetry = app.symmetry.toggle_horizontal();
            app.set_status(&format!("Symmetry: {}", app.symmetry.label()));
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.symmetry = app.symmetry.toggle_vertical();
            app.set_status(&format!("Symmetry: {}", app.symmetry.label()));
        }

        // Preview toggle
        KeyCode::Tab => {
            app.show_preview = !app.show_preview;
        }

        // Quick color pick: 1-9 → curated palette slots 0-8, 0 → slot 9
        KeyCode::Char(c @ '1'..='9') => {
            let n = (c as u8 - b'1') as usize;
            app.quick_pick_color(n);
        }
        KeyCode::Char('0') => {
            app.quick_pick_color(9);
        }

        // Palette navigation (uses palette_layout)
        KeyCode::Up => {
            if app.palette_cursor > 0 {
                app.palette_cursor -= 1;
                if let Some(PaletteItem::Color(idx)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = Color256(*idx);
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        KeyCode::Down => {
            if app.palette_cursor + 1 < app.palette_layout.len() {
                app.palette_cursor += 1;
                if let Some(PaletteItem::Color(idx)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = Color256(*idx);
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        KeyCode::Left => {
            if app.palette_cursor >= 6 {
                app.palette_cursor -= 6;
                if let Some(PaletteItem::Color(idx)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = Color256(*idx);
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        KeyCode::Right => {
            if app.palette_cursor + 6 < app.palette_layout.len() {
                app.palette_cursor += 6;
                if let Some(PaletteItem::Color(idx)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = Color256(*idx);
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        // Enter on palette: toggle section header or select color
        KeyCode::Enter => {
            if let Some(item) = app.palette_layout.get(app.palette_cursor).copied() {
                match item {
                    PaletteItem::SectionHeader(section) => {
                        match section {
                            PaletteSection::Standard => {
                                app.palette_sections.standard_expanded = !app.palette_sections.standard_expanded;
                            }
                            PaletteSection::HueGroups => {
                                app.palette_sections.hue_expanded = !app.palette_sections.hue_expanded;
                            }
                            PaletteSection::Grayscale => {
                                app.palette_sections.grayscale_expanded = !app.palette_sections.grayscale_expanded;
                            }
                        }
                        app.rebuild_palette_layout();
                        // Clamp cursor if layout shrank
                        if app.palette_cursor >= app.palette_layout.len() {
                            app.palette_cursor = app.palette_layout.len().saturating_sub(1);
                        }
                    }
                    PaletteItem::Color(idx) => {
                        app.color = Color256(idx);
                    }
                }
            }
        }

        // HSL color sliders
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let (r, g, b) = app.color.to_rgb();
            let (h, s, l) = crate::palette::rgb_to_hsl(r, g, b);
            app.slider_h = h;
            app.slider_s = s;
            app.slider_l = l;
            app.slider_active = 0;
            app.mode = AppMode::ColorSliders;
        }

        // Custom palette dialog
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.open_palette_dialog();
        }

        // Add current color to active custom palette
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.add_color_to_custom_palette();
        }

        // Cycle block character type
        KeyCode::Char('b') | KeyCode::Char('B') => {
            app.cycle_block();
        }

        // Toggle filled/outline rectangle
        KeyCode::Char('t') | KeyCode::Char('T') => {
            app.filled_rect = !app.filled_rect;
            app.set_status(if app.filled_rect { "Rect: Filled" } else { "Rect: Outline" });
        }

        // Cancel multi-click tool
        KeyCode::Esc => {
            app.cancel_tool();
            app.set_status("Cancelled");
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }

        // Quit
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            if app.dirty {
                app.mode = AppMode::Quitting;
                app.set_status("Unsaved changes. Quit? (y/n)");
            } else {
                app.running = false;
            }
        }

        _ => {}
    }
}

fn handle_file_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.file_dialog_selected > 0 {
                app.file_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.file_dialog_selected + 1 < app.file_dialog_files.len() {
                app.file_dialog_selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(filename) = app.file_dialog_files.get(app.file_dialog_selected).cloned() {
                app.mode = AppMode::Normal;
                app.load_project(&filename);
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_export_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.export_cursor > 0 {
                app.export_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.export_cursor < 1 {
                app.export_cursor += 1;
            }
        }
        KeyCode::Left | KeyCode::Right => {
            if app.export_cursor == 0 {
                app.export_format = 1 - app.export_format;
            } else {
                app.export_dest = 1 - app.export_dest;
            }
        }
        KeyCode::Enter => {
            app.do_export();
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

enum TextInputPurpose {
    SaveAs,
    ExportFile,
    PaletteName,
    PaletteRename,
    PaletteExport,
}

fn handle_text_input(app: &mut App, key: KeyEvent, purpose: TextInputPurpose) {
    match key.code {
        KeyCode::Enter => {
            let input = app.text_input.clone();
            if input.trim().is_empty() {
                app.set_status("Name cannot be empty");
                return;
            }
            match purpose {
                TextInputPurpose::SaveAs => {
                    app.mode = AppMode::Normal;
                    app.save_as(input.trim());
                }
                TextInputPurpose::ExportFile => {
                    app.export_to_file(input.trim());
                }
                TextInputPurpose::PaletteName => {
                    app.create_custom_palette(input.trim());
                }
                TextInputPurpose::PaletteRename => {
                    app.rename_selected_palette(input.trim());
                }
                TextInputPurpose::PaletteExport => {
                    app.export_selected_palette(input.trim());
                }
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            app.text_input.pop();
        }
        KeyCode::Char(c) => {
            if app.text_input.len() < 64 {
                app.text_input.push(c);
            }
        }
        _ => {}
    }
}

fn handle_color_sliders(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.slider_active > 0 {
                app.slider_active -= 1;
            }
        }
        KeyCode::Down => {
            if app.slider_active < 2 {
                app.slider_active += 1;
            }
        }
        KeyCode::Left => {
            match app.slider_active {
                0 => app.slider_h = app.slider_h.saturating_sub(5),
                1 => app.slider_s = app.slider_s.saturating_sub(5),
                _ => app.slider_l = app.slider_l.saturating_sub(5),
            }
        }
        KeyCode::Right => {
            match app.slider_active {
                0 => app.slider_h = (app.slider_h + 5).min(359),
                1 => app.slider_s = (app.slider_s + 5).min(100),
                _ => app.slider_l = (app.slider_l + 5).min(100),
            }
        }
        KeyCode::Enter => {
            let (r, g, b) = crate::palette::hsl_to_rgb(app.slider_h, app.slider_s, app.slider_l);
            let color = crate::palette::nearest_color(r, g, b);
            app.color = color;
            app.mode = AppMode::Normal;
            app.set_status(&format!("Color: {}", color.name()));
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_palette_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.palette_dialog_selected > 0 {
                app.palette_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.palette_dialog_selected + 1 < app.palette_dialog_files.len() {
                app.palette_dialog_selected += 1;
            }
        }
        KeyCode::Enter => {
            app.load_selected_palette();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.text_input = String::new();
            app.mode = AppMode::PaletteNameInput;
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.delete_selected_palette();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if !app.palette_dialog_files.is_empty() {
                // Pre-fill with current name (without .palette extension)
                if let Some(filename) = app.palette_dialog_files.get(app.palette_dialog_selected) {
                    app.text_input = filename.trim_end_matches(".palette").to_string();
                }
                app.mode = AppMode::PaletteRename;
            }
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            app.duplicate_selected_palette();
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if !app.palette_dialog_files.is_empty() {
                if let Some(filename) = app.palette_dialog_files.get(app.palette_dialog_selected) {
                    app.text_input = filename.clone();
                }
                app.mode = AppMode::PaletteExport;
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_new_canvas(app: &mut App, code: KeyCode) {
    use crate::canvas::{MIN_DIMENSION, MAX_DIMENSION};

    match code {
        KeyCode::Up | KeyCode::Down => {
            app.new_canvas_cursor = 1 - app.new_canvas_cursor;
        }
        KeyCode::Left => {
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = app.new_canvas_width.saturating_sub(8).max(MIN_DIMENSION);
            } else {
                app.new_canvas_height = app.new_canvas_height.saturating_sub(8).max(MIN_DIMENSION);
            }
        }
        KeyCode::Right => {
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = (app.new_canvas_width + 8).min(MAX_DIMENSION);
            } else {
                app.new_canvas_height = (app.new_canvas_height + 8).min(MAX_DIMENSION);
            }
        }
        KeyCode::Enter => {
            let w = app.new_canvas_width;
            let h = app.new_canvas_height;
            app.canvas = Canvas::new_with_size(w, h);
            app.history = History::new();
            app.dirty = false;
            app.project_name = None;
            app.project_path = None;
            app.cursor = None;
            app.tool_state = ToolState::Idle;
            app.mode = AppMode::Normal;
            app.set_status(&format!("New canvas {}x{}", w, h));
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, canvas_area: &CanvasArea) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                app.cursor = Some((x, y));
                // Start stroke for continuous tools
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.begin_stroke();
                }
                app.apply_tool(x, y);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                app.cursor = Some((x, y));
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.apply_tool(x, y);
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app.history.is_stroke_active() {
                app.end_stroke();
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            // Quick eyedropper
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                if let Some((picked, _bg, block)) = crate::tools::eyedropper(&app.canvas, x, y) {
                    app.color = picked;
                    if block != crate::cell::BlockChar::Empty {
                        app.active_block = block;
                    }
                    app.set_status(&format!("Picked: {} {}", picked.name(), block.to_char()));
                }
            }
        }
        MouseEventKind::Moved => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                app.cursor = Some((x, y));
            } else {
                app.cursor = None;
            }
        }
        _ => {}
    }
}
