use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::tools::ToolKind;

/// Tool list: 6 tool entries.
pub fn tool_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let mut lines: Vec<Line> = Vec::new();

    for tool in ToolKind::ALL {
        let is_active = app.active_tool == tool;
        let prefix = if is_active { "\u{25B8}" } else { " " }; // ▸ or space
        let style = if is_active {
            Style::default()
                .fg(Color::Indexed(16))
                .bg(theme.highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!(" {}{} {} {}", prefix, tool.key(), tool.icon(), tool.name()),
            style,
        )));
    }

    lines
}

/// Symmetry toggle row: [H] [V].
pub fn symmetry_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let sym = app.symmetry;
    let h_style = if sym.has_horizontal() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };
    let v_style = if sym.has_vertical() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };

    vec![Line::from(vec![
        Span::styled(" [H] ", h_style),
        Span::styled("[V]", v_style),
    ])]
}

/// Block cycle + rect fill/outline toggle.
pub fn block_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let block_line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            format!("{}", app.active_block),
            Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " [B] Cycle",
            Style::default().fg(theme.dim),
        ),
    ]);

    let rect_text = if app.filled_rect { " [T] Filled" } else { " [T] Outline" };
    let rect_line = Line::from(Span::styled(rect_text, Style::default().fg(theme.dim)));

    vec![block_line, rect_line]
}

/// Active color swatch display.
pub fn color_swatch_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let label = Line::from(Span::styled(
        " Color:",
        Style::default().fg(theme.accent),
    ));
    let swatch = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            "    ",
            Style::default().bg(app.color.to_ratatui()),
        ),
        Span::styled(
            format!(" {}", app.color.name()),
            Style::default().fg(theme.dim),
        ),
    ]);
    vec![label, swatch]
}
