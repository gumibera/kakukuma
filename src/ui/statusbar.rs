use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let mut spans = Vec::new();

    // Status message takes priority
    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" {} ", msg.text),
            Style::default().fg(theme.highlight).bg(theme.panel_bg),
        ));
    } else {
        // Default shortcut hints — dim undo/redo when unavailable
        let undo_fg = if app.history.can_undo() { Color::White } else { theme.dim };
        let undo_label_fg = if app.history.can_undo() { Color::Gray } else { theme.dim };
        let redo_fg = if app.history.can_redo() { Color::White } else { theme.dim };
        let redo_label_fg = if app.history.can_redo() { Color::Gray } else { theme.dim };

        let sep_style = Style::default().fg(theme.separator).bg(theme.panel_bg);

        // Left group: file + edit
        for &(key, label, key_fg, label_fg) in &[
            ("^S", " Save ", Color::White, Color::Gray),
            ("^O", " Open ", Color::White, Color::Gray),
            ("^E", " Export ", Color::White, Color::Gray),
        ] {
            spans.push(Span::styled(key, Style::default().fg(key_fg).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(label_fg).bg(theme.panel_bg)));
        }

        spans.push(Span::styled(" \u{2502} ", sep_style));

        for &(key, label, key_fg, label_fg) in &[
            ("^Z", " Undo ", undo_fg, undo_label_fg),
            ("^Y", " Redo ", redo_fg, redo_label_fg),
        ] {
            spans.push(Span::styled(key, Style::default().fg(key_fg).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(label_fg).bg(theme.panel_bg)));
        }

        // Right group: help, quit, cursor position
        let mut right_spans: Vec<Span> = Vec::new();
        for &(key, label) in &[("?", " Help "), ("Q", " Quit ")] {
            right_spans.push(Span::styled(key, Style::default().fg(Color::White).bg(theme.panel_bg)));
            right_spans.push(Span::styled(label, Style::default().fg(Color::Gray).bg(theme.panel_bg)));
        }
        if let Some((x, y)) = app.cursor {
            right_spans.push(Span::styled(
                format!("({},{}) ", x, y),
                Style::default().fg(Color::Cyan).bg(theme.panel_bg),
            ));
        }

        let left_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let right_width: usize = right_spans.iter().map(|s| s.content.len()).sum();
        let padding = (area.width as usize).saturating_sub(left_width + right_width);
        spans.push(Span::styled(
            " ".repeat(padding),
            Style::default().bg(theme.panel_bg),
        ));
        spans.extend(right_spans);
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.panel_bg));
    f.render_widget(paragraph, area);
}
