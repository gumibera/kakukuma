pub mod editor;
pub mod toolbar;
pub mod palette;
pub mod statusbar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, AppMode};
use crate::input::CanvasArea;
use crate::theme::Theme;

/// Render the full UI and return the canvas area for mouse mapping.
pub fn render(f: &mut Frame, app: &App) -> CanvasArea {
    let size = f.area();
    let theme = app.theme();

    // Check minimum size
    if size.width < 100 || size.height < 36 {
        let msg = Paragraph::new(format!(
            "Terminal too small: {}x{}\nMinimum: 100x36\nPlease resize.",
            size.width, size.height
        ))
        .style(Style::default().fg(Color::Red));
        f.render_widget(msg, size);
        return CanvasArea {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        };
    }

    // Top-level: main bordered frame + status bar outside
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(34),   // Main frame
            Constraint::Length(1), // Status bar (outside border)
        ])
        .split(size);

    let main_area = outer[0];
    let status_area = outer[1];

    // Render the main border frame
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.separator));
    let inner = main_block.inner(main_area).inner(Margin::new(1, 0));
    f.render_widget(main_block, main_area);

    // Inside the frame: header + body
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(32),  // Body
        ])
        .split(inner);

    let header_area = vertical[0];
    let body_area = vertical[1];

    // Header
    render_header(f, app, header_area, theme);

    // Body: left toolbar | canvas | right palette
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .spacing(1)
        .constraints([
            Constraint::Length(14), // Toolbar (bordered panel)
            Constraint::Min(60),   // Canvas (reduced for margin+spacing)
            Constraint::Length(20), // Palette (bordered panel)
        ])
        .split(body_area);

    let toolbar_area = horizontal[0];
    let canvas_area = horizontal[1];
    let palette_area = horizontal[2];

    // Toolbar (3 boxes)
    let tool_lines = toolbar::tool_lines(app);
    let sym_lines = toolbar::symmetry_lines(app);
    let blk_lines = toolbar::block_lines(app);
    render_box_column(f, toolbar_area, &[
        BoxContent { title: " \u{2022} Tools \u{2022} ", lines: &tool_lines },
        BoxContent { title: " \u{2022} Symmetry \u{2022} ", lines: &sym_lines },
        BoxContent { title: " \u{2022} Block \u{2022} ", lines: &blk_lines },
    ], theme);

    // Canvas
    let canvas_screen_area = if app.show_preview && size.width >= 160 {
        let side_by_side = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 2),
                Constraint::Ratio(1, 2),
            ])
            .split(canvas_area);
        let editor_area = editor::render(f, app, side_by_side[0]);
        editor::render_preview(f, app, side_by_side[1]);
        editor_area
    } else if app.show_preview {
        editor::render_preview(f, app, canvas_area);
        CanvasArea {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        }
    } else {
        editor::render(f, app, canvas_area)
    };

    // Palette (3 boxes)
    let colors_lines = palette::color_lines(app);
    let section_lines = palette::section_lines(app);
    let info_lines = palette::info_lines(app);
    let section_title = if let Some(ref cp) = app.custom_palette {
        format!(" \u{2022} {} \u{2022} ", cp.name)
    } else {
        " \u{2022} Sections \u{2022} ".to_string()
    };
    render_palette_column(
        f, palette_area,
        &colors_lines, &section_lines, &info_lines,
        &section_title, app.palette_scroll, theme,
    );

    // Status bar (outside the border)
    statusbar::render(f, app, status_area);

    // Overlays
    match app.mode {
        AppMode::Help => render_help(f, app, size),
        AppMode::Quitting => render_quit_prompt(f, size),
        AppMode::FileDialog => render_file_dialog(f, app, size),
        AppMode::ExportDialog => render_export_dialog(f, app, size),
        AppMode::SaveAs => render_text_input(f, app, size, "Save As", "Enter project name:"),
        AppMode::ExportFile => render_text_input(f, app, size, "Export", "Enter filename:"),
        AppMode::Recovery => render_recovery_prompt(f, app, size),
        AppMode::ColorSliders => render_color_sliders(f, app, size),
        AppMode::PaletteDialog => render_palette_dialog(f, app, size),
        AppMode::PaletteNameInput => render_text_input(f, app, size, "New Palette", "Enter palette name:"),
        AppMode::PaletteRename => render_text_input(f, app, size, "Rename Palette", "Enter new name:"),
        AppMode::PaletteExport => render_text_input(f, app, size, "Export Palette", "Enter destination path:"),
        AppMode::NewCanvas => render_new_canvas(f, app, size),
        _ => {}
    }

    canvas_screen_area
}

struct BoxContent<'a> {
    title: &'a str,
    lines: &'a [ratatui::text::Line<'static>],
}

/// Render N bordered boxes evenly distributed vertically in a column.
fn render_box_column(
    f: &mut Frame,
    column: Rect,
    boxes: &[BoxContent],
    theme: &Theme,
) {
    let n = boxes.len() as u16;
    let box_heights: Vec<u16> = boxes.iter()
        .map(|b| b.lines.len() as u16 + 2)
        .collect();
    let total_box_height: u16 = box_heights.iter().sum();
    let remaining = column.height.saturating_sub(total_box_height);
    let gap_count = n + 1;
    let gap = remaining / gap_count.max(1);

    let mut y = column.y + gap;
    for (i, bx) in boxes.iter().enumerate() {
        let h = box_heights[i];
        let area = Rect::new(column.x, y, column.width, h);
        render_bordered_panel(f, area, bx.lines, bx.title, theme);
        y += h + gap;
    }
}

/// Render 3 palette boxes: Colors (fixed), Sections (scrollable), Color info (fixed).
#[allow(clippy::too_many_arguments)]
fn render_palette_column(
    f: &mut Frame,
    column: Rect,
    colors_lines: &[ratatui::text::Line<'static>],
    section_lines: &[ratatui::text::Line<'static>],
    info_lines: &[ratatui::text::Line<'static>],
    section_title: &str,
    scroll: usize,
    theme: &Theme,
) {
    let colors_height = colors_lines.len() as u16 + 2;
    let info_height = info_lines.len() as u16 + 2;
    let gap_count = 4u16; // 3 boxes → 4 gaps
    let section_content_height = section_lines.len() as u16;

    // Sections box gets remaining space after other boxes and gaps
    let section_max = column.height
        .saturating_sub(colors_height + info_height + gap_count);
    let section_box_height = (section_content_height + 2)
        .min(section_max)
        .max(5); // minimum 5 rows (3 headers + border)

    let total_box_height = colors_height + section_box_height + info_height;
    let remaining = column.height.saturating_sub(total_box_height);
    let gap = remaining / gap_count.max(1);

    let mut y = column.y + gap;

    // Colors box
    let colors_area = Rect::new(column.x, y, column.width, colors_height);
    render_bordered_panel(f, colors_area, colors_lines, " \u{2022} Colors \u{2022} ", theme);
    y += colors_height + gap;

    // Sections box (scrollable)
    let section_area = Rect::new(column.x, y, column.width, section_box_height);
    render_bordered_panel_scrollable(f, section_area, section_lines, section_title, scroll, theme);
    y += section_box_height + gap;

    // Color info box
    let info_area = Rect::new(column.x, y, column.width, info_height);
    render_bordered_panel(f, info_area, info_lines, " \u{2022} Color \u{2022} ", theme);
}

/// Render content lines inside a vertically-centered bordered panel.
fn render_bordered_panel(
    f: &mut Frame,
    column: Rect,
    lines: &[ratatui::text::Line<'static>],
    title: &str,
    theme: &Theme,
) {
    let content_height = lines.len() as u16;
    let panel_height = (content_height + 2).min(column.height); // +2 for border
    let offset_y = (column.height.saturating_sub(panel_height)) / 2;

    let panel_area = Rect::new(
        column.x,
        column.y + offset_y,
        column.width,
        panel_height,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_accent))
        .title(ratatui::text::Span::styled(
            title.to_string(),
            Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines.to_vec()).block(block);
    f.render_widget(paragraph, panel_area);
}

/// Render content lines inside a bordered panel with scroll support.
fn render_bordered_panel_scrollable(
    f: &mut Frame,
    column: Rect,
    lines: &[ratatui::text::Line<'static>],
    title: &str,
    scroll: usize,
    theme: &Theme,
) {
    let content_height = lines.len() as u16;
    let inner_height = column.height.saturating_sub(2); // available inside border

    let (panel_area, scroll_offset) = if content_height <= inner_height {
        // Content fits — center the panel
        let panel_height = content_height + 2;
        let offset_y = (column.height.saturating_sub(panel_height)) / 2;
        (
            Rect::new(column.x, column.y + offset_y, column.width, panel_height),
            0u16,
        )
    } else {
        // Content overflows — fill column, apply scroll
        (column, scroll as u16)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_accent))
        .title(ratatui::text::Span::styled(
            title.to_string(),
            Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines.to_vec())
        .block(block)
        .scroll((scroll_offset, 0));
    f.render_widget(paragraph, panel_area);
}

fn render_header(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let name = app
        .project_name
        .as_deref()
        .unwrap_or("untitled");
    let dirty_marker = if app.dirty { "*" } else { "" };
    let tool_name = app.active_tool.name();
    let sym = app.symmetry.label();

    let header_text = format!(
        " \u{0295}\u{2022}\u{1d25}\u{2022}\u{0294} kakukuma \u{2014} {}{} {:>width$}",
        name,
        dirty_marker,
        format!("Tool: {}  Sym: {}", tool_name, sym),
        width = (area.width as usize).saturating_sub(name.len() + dirty_marker.len() + 22)
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::White).bg(theme.header_bg));
    f.render_widget(header, area);
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::Span;
    let theme = app.theme();

    let sep = Style::default().fg(theme.separator).bg(theme.panel_bg);
    let hdr = Style::default().fg(theme.accent).bg(theme.panel_bg);
    let txt = Style::default().fg(Color::White).bg(theme.panel_bg);
    let dim = Style::default().fg(theme.dim).bg(theme.panel_bg);

    let lines: Vec<ratatui::text::Line> = vec![
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(vec![
            Span::styled("  Tools", hdr),
            Span::styled("                Canvas", hdr),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
            Span::styled("                \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  P  Pencil", txt),
            Span::styled("         Tab  Preview", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  E  Eraser", txt),
            Span::styled("         B    Cycle block", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  L  Line", txt),
            Span::styled("           T    Rect fill/outline", txt),
        ]),
        ratatui::text::Line::from(Span::styled("  R  Rectangle", txt)),
        ratatui::text::Line::from(vec![
            Span::styled("  F  Fill", txt),
            Span::styled("           Symmetry", hdr),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  I  Eyedropper", txt),
            Span::styled("     \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("                    ", txt),
            Span::styled("H  Horizontal mirror", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  Colors", hdr),
            Span::styled("            V  Vertical mirror", txt),
        ]),
        ratatui::text::Line::from(Span::styled("  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep)),
        ratatui::text::Line::from(vec![
            Span::styled("  1-0  Quick pick", txt),
            Span::styled("   File", hdr),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  S    HSL sliders", txt),
            Span::styled("  \u{2500}\u{2500}\u{2500}\u{2500}", sep),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  A    Add color", txt),
            Span::styled("    ^S Save  ^O Open", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  C    Palettes", txt),
            Span::styled("     ^N New   ^E Export", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("                    ", txt),
            Span::styled("^Z Undo  ^Y Redo", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("                    ", txt),
            Span::styled("^T Theme  Q Quit  ? Help", txt),
        ]),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(Span::styled(
            "         Press any key to close",
            dim,
        )),
    ];

    let width = 48;
    let height = lines.len() as u16 + 2;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let help_area = Rect::new(x, y, width, height);

    let help = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Help ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(help, help_area);
}

fn render_quit_prompt(f: &mut Frame, area: Rect) {
    let width = 40;
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let prompt_area = Rect::new(x, y, width, height);

    let prompt = Paragraph::new(" Unsaved changes. Quit? (y/n)")
        .style(Style::default().fg(Color::White).bg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Quit ")
                .style(Style::default().fg(Color::White).bg(Color::Red)),
        );
    f.render_widget(prompt, prompt_area);
}

fn render_file_dialog(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let file_count = app.file_dialog_files.len();
    let height = (file_count as u16 + 4).min(20);
    let width = 44;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();
    let visible_start = if app.file_dialog_selected > (height as usize).saturating_sub(5) {
        app.file_dialog_selected - (height as usize).saturating_sub(5)
    } else {
        0
    };

    for (i, filename) in app.file_dialog_files.iter().enumerate().skip(visible_start) {
        if lines.len() >= (height as usize).saturating_sub(4) {
            break;
        }
        let is_selected = i == app.file_dialog_selected;
        let prefix = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(theme.highlight)
        } else {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        };
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            format!("{}{}", prefix, filename),
            style,
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Navigate  Enter Open  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Open File ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_export_dialog(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let width = 40;
    let height = 10;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let format_opts = ["Unicode", "ANSI Art"];
    let dest_opts = ["Clipboard", "File"];

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    // Format row
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Format:",
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    let mut fmt_spans = Vec::new();
    fmt_spans.push(ratatui::text::Span::raw("  "));
    for (i, opt) in format_opts.iter().enumerate() {
        let selected = i == app.export_format;
        let focused = app.export_cursor == 0;
        let style = if selected && focused {
            Style::default().fg(Color::Black).bg(theme.highlight)
        } else if selected {
            Style::default().fg(Color::Black).bg(Color::Gray)
        } else {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        };
        fmt_spans.push(ratatui::text::Span::styled(format!(" {} ", opt), style));
        if i == 0 {
            fmt_spans.push(ratatui::text::Span::raw(" "));
        }
    }
    lines.push(ratatui::text::Line::from(fmt_spans));
    lines.push(ratatui::text::Line::from(""));

    // Destination row
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Destination:",
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    let mut dest_spans = Vec::new();
    dest_spans.push(ratatui::text::Span::raw("  "));
    for (i, opt) in dest_opts.iter().enumerate() {
        let selected = i == app.export_dest;
        let focused = app.export_cursor == 1;
        let style = if selected && focused {
            Style::default().fg(Color::Black).bg(theme.highlight)
        } else if selected {
            Style::default().fg(Color::Black).bg(Color::Gray)
        } else {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        };
        dest_spans.push(ratatui::text::Span::styled(format!(" {} ", opt), style));
        if i == 0 {
            dest_spans.push(ratatui::text::Span::raw(" "));
        }
    }
    lines.push(ratatui::text::Line::from(dest_spans));
    lines.push(ratatui::text::Line::from(""));

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Row  \u{2190}\u{2192} Option  Enter Go  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Export ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_text_input(f: &mut Frame, app: &App, area: Rect, title: &str, prompt: &str) {
    let theme = app.theme();
    let width = 44;
    let height = 7;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}", prompt),
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}\u{2588}", app.text_input),
        Style::default().fg(Color::White).bg(Color::Black),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Enter Confirm  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", title))
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_recovery_prompt(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let width = 44;
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let prompt_area = Rect::new(x, y, width, height);

    let prompt = Paragraph::new(" Autosave found. Recover? (y/n)")
        .style(Style::default().fg(Color::White).bg(theme.border_accent))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Recovery ")
                .style(Style::default().fg(Color::White).bg(theme.border_accent)),
        );
    f.render_widget(prompt, prompt_area);
}

fn render_color_sliders(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let width = 44;
    let height = 14;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let bar_width = 20;
    let sliders: [(&str, u16, u16); 3] = [
        ("H", app.slider_h, 359),
        ("S", app.slider_s as u16, 100),
        ("L", app.slider_l as u16, 100),
    ];

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    for (i, (label, value, max_val)) in sliders.iter().enumerate() {
        let is_active = i as u8 == app.slider_active;
        let filled = (*value as usize * bar_width) / (*max_val as usize).max(1);
        let empty = bar_width - filled;
        let bar: String = format!(
            "{}{}",
            "\u{2588}".repeat(filled),
            "\u{2591}".repeat(empty),
        );

        let label_style = if is_active {
            Style::default().fg(theme.accent).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim)
        };

        let bar_style = if is_active {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        } else {
            Style::default().fg(theme.dim).bg(theme.panel_bg)
        };

        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!(" {} ", label), label_style),
            ratatui::text::Span::styled(bar, bar_style),
            ratatui::text::Span::styled(
                format!(" {:>3}", value),
                Style::default().fg(Color::White).bg(theme.panel_bg),
            ),
        ]));
    }

    lines.push(ratatui::text::Line::from(""));

    // Live preview
    let (r, g, b) = crate::palette::hsl_to_rgb(app.slider_h, app.slider_s, app.slider_l);
    let preview_color = crate::palette::nearest_color(r, g, b);
    let preview_rcolor = preview_color.to_ratatui();

    lines.push(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(" Preview: ", Style::default().fg(theme.dim).bg(theme.panel_bg)),
        ratatui::text::Span::styled(
            "\u{2588}\u{2588}\u{2588}\u{2588}",
            Style::default().fg(preview_rcolor).bg(theme.panel_bg),
        ),
        ratatui::text::Span::styled(
            format!("  {}", preview_color.name()),
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        ),
    ]));

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" RGB: ({}, {}, {})  #{}", r, g, b, preview_color.0),
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Slider  \u{2190}\u{2192} Adjust  Enter Apply  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Color Sliders ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_palette_dialog(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let file_count = app.palette_dialog_files.len();
    let height = (file_count as u16 + 8).min(22);
    let width = 44;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    if app.palette_dialog_files.is_empty() {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            " No palettes found",
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        )));
    } else {
        let visible_start = if app.palette_dialog_selected > (height as usize).saturating_sub(7) {
            app.palette_dialog_selected - (height as usize).saturating_sub(7)
        } else {
            0
        };

        for (i, filename) in app.palette_dialog_files.iter().enumerate().skip(visible_start) {
            if lines.len() >= (height as usize).saturating_sub(6) {
                break;
            }
            let is_selected = i == app.palette_dialog_selected;
            let prefix = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(theme.highlight)
            } else {
                Style::default().fg(Color::White).bg(theme.panel_bg)
            };
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("{}{}", prefix, filename),
                style,
            )));
        }
    }

    // Show active palette
    if let Some(ref cp) = app.custom_palette {
        lines.push(ratatui::text::Line::from(""));
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            format!(" Active: {} ({} colors)", cp.name, cp.colors.len()),
            Style::default().fg(theme.accent).bg(theme.panel_bg),
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Nav  Enter Load  N New",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " R Rename  U Dup  D Del",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " X Export  Esc Close",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Custom Palettes ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_new_canvas(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Clear;

    let theme = app.theme();
    let w = 30u16;
    let h = 8u16;
    let dialog_area = Rect::new(
        area.width.saturating_sub(w) / 2,
        area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    );
    f.render_widget(Clear, dialog_area);

    let w_style = if app.new_canvas_cursor == 0 {
        Style::default().fg(Color::Black).bg(theme.highlight).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let h_style = if app.new_canvas_cursor == 1 {
        Style::default().fg(Color::Black).bg(theme.highlight).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let dim = Style::default().fg(theme.dim);

    let lines = vec![
        Line::from(vec![
            Span::styled(" Width:  ", dim),
            Span::styled(format!("\u{25C0} {:>3} \u{25B6}", app.new_canvas_width), w_style),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled(" Height: ", dim),
            Span::styled(format!("\u{25C0} {:>3} \u{25B6}", app.new_canvas_height), h_style),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" Enter=Create  Esc=Cancel", dim)),
    ];

    let dialog = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" New Canvas ")
            .style(Style::default().fg(theme.accent).bg(theme.panel_bg)),
    );
    f.render_widget(dialog, dialog_area);
}
