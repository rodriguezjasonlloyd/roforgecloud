use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, TextFieldExt, TreeTarget};
use crate::json_tree;
use crate::screens;
use crate::status;

pub(crate) const HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Rgb(60, 60, 60)).fg(Color::White);

pub(crate) fn breadcrumb(parts: &[&str], suffix: Option<&str>) -> Line<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, &part) in parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" > ", dim));
        }
        spans.push(Span::raw(part.to_string()));
    }
    if let Some(s) = suffix {
        spans.push(Span::styled(format!("  {s}"), dim));
    }
    Line::from(spans)
}

pub(crate) fn universe_label(app: &App) -> String {
    app.universe_names
        .get(&app.universe_id)
        .cloned()
        .unwrap_or_else(|| app.universe_id.to_string())
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    (screens::def(app.screen).draw)(frame, app, chunks[0]);

    draw_status(frame, app, chunks[1]);

    if app.which_key.active {
        draw_help(frame, app);
    }
}

fn scalar_style(kind: &json_tree::ScalarKind) -> Style {
    match kind {
        json_tree::ScalarKind::String => Style::default().fg(Color::Green),
        json_tree::ScalarKind::Bool => Style::default().fg(Color::Magenta),
        json_tree::ScalarKind::Null => Style::default().fg(Color::DarkGray),
        json_tree::ScalarKind::Number => Style::default().fg(Color::Yellow),
    }
}

pub(crate) fn draw_tree(frame: &mut Frame, app: &App, area: Rect) {
    let Some(editor) = &app.tree_editor else {
        return;
    };
    let rows = json_tree::flatten(editor.root());

    let mut edit_cursor_col = 0u16;
    let mut edit_value_prefix_col = 0u16;

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut spans = vec![Span::raw("  ".repeat(row.depth))];

            if row.is_container {
                let marker = if row.is_collapsed { "▸ " } else { "▾ " };
                spans.push(Span::styled(marker, Style::default().fg(Color::DarkGray)));
            } else {
                spans.push(Span::raw("  "));
            }

            if i == editor.cursor() && editor.editing_key() {
                let field = editor.edit_key();
                let prefix_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                let cursor_idx = field.cursor().1;
                edit_cursor_col = (prefix_width + 1 + cursor_idx) as u16;
                let style = Style::new().bg(Color::Rgb(50, 50, 50)).fg(Color::Yellow);
                spans.push(Span::raw("\""));
                spans.push(Span::styled(field.get_value().to_string(), style));
                spans.push(Span::raw("\": "));
            } else if let Some(key) = &row.key {
                spans.push(Span::styled(
                    format!("{key:?}: "),
                    Style::default().fg(Color::Cyan),
                ));
            }

            if i == editor.cursor() && editor.editing() {
                edit_value_prefix_col = spans.iter().map(|s| s.content.chars().count()).sum::<usize>() as u16;
            } else {
                let style = row.scalar_kind.as_ref().map(scalar_style).unwrap_or_default();
                spans.push(Span::styled(row.preview.clone(), style));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let editing = editor.is_editing();

    let uni = universe_label(app);
    let mode = if editing { "editing" } else { "tree" };
    let title = match app.tree_target {
        TreeTarget::Value => {
            use crate::app::ValueSource;
            match app.value.source {
                ValueSource::DataStore => breadcrumb(
                    &[uni.as_str(), "data stores", &app.stores.data_store_id, &app.value.title],
                    Some(mode),
                ),
                ValueSource::MemoryStoreSortedMap => breadcrumb(
                    &[uni.as_str(), "memory stores", &app.memory_store_input.id, &app.memory_item_editing_id],
                    Some(mode),
                ),
            }
        }
        TreeTarget::EntriesCreate | TreeTarget::MemoryCreate => {
            breadcrumb(&["value"], Some(mode))
        }
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if editing {
        state.select(None);
        let visible_height = area.height.saturating_sub(2) as usize;
        let max_offset = rows.len().saturating_sub(visible_height);
        *state.offset_mut() = editor
            .cursor()
            .saturating_sub(visible_height / 2)
            .min(max_offset);
    } else {
        state.select(Some(editor.cursor()));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if editor.editing_key() {
        let inner = Block::default().borders(Borders::ALL).inner(area);
        let row_y = editor.cursor().saturating_sub(state.offset());
        if row_y < inner.height as usize {
            frame.set_cursor_position((inner.x + edit_cursor_col, inner.y + row_y as u16));
        }
    } else if editor.editing() {
        let inner = Block::default().borders(Borders::ALL).inner(area);
        let row_y = editor.cursor().saturating_sub(state.offset());
        if row_y < inner.height as usize {
            let textarea = editor.edit_text();
            let lines = textarea.lines().len().max(1) as u16;
            let height = lines.min(inner.height.saturating_sub(row_y as u16));
            let overlay = Rect {
                x: inner.x + edit_value_prefix_col,
                y: inner.y + row_y as u16,
                width: inner.width.saturating_sub(edit_value_prefix_col),
                height,
            };
            frame.render_widget(Clear, overlay);
            frame.render_widget(textarea, overlay);
        }
    }
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if app.loading {
        (status::loading().text, Style::default())
    } else {
        let color = match app.status.kind {
            status::Kind::Ok => Style::default().fg(Color::Green),
            status::Kind::Err => Style::default().fg(Color::Red),
            status::Kind::Info => Style::default(),
        };
        (app.status.text.clone(), color)
    };
    let paragraph = Paragraph::new(text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}


pub(crate) fn field_box(frame: &mut Frame, area: Rect, title: &str, field: &tui_textarea::TextArea<'static>, active: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (line, cursor_col) = field_line(field, inner.width);
    frame.render_widget(Paragraph::new(line), inner);
    if active {
        frame.set_cursor_position((inner.x + cursor_col, inner.y));
    }
}

pub(crate) fn field_paragraph_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    field: &tui_textarea::TextArea<'static>,
    active: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (lines, cursor) = field_paragraph(field, inner.width, inner.height);
    frame.render_widget(Paragraph::new(lines), inner);
    if active {
        frame.set_cursor_position((inner.x + cursor.0, inner.y + cursor.1));
    }
}

fn field_line(field: &tui_textarea::TextArea<'static>, width: u16) -> (Line<'static>, u16) {
    let field_width = width.max(1) as usize;
    let value = field.get_value();
    let chars: Vec<char> = value.chars().collect();
    let cursor_idx = field.cursor().1;

    let start = if cursor_idx >= field_width {
        cursor_idx + 1 - field_width
    } else {
        0
    };
    let end = (start + field_width).min(chars.len());
    let visible: Vec<char> = chars[start..end].to_vec();

    let mut text: String = visible.into_iter().collect();
    while text.chars().count() < field_width {
        text.push(' ');
    }
    let cursor_col = (cursor_idx - start) as u16;
    (Line::from(Span::raw(text)), cursor_col)
}

fn field_paragraph(
    field: &tui_textarea::TextArea<'static>,
    width: u16,
    max_lines: u16,
) -> (Vec<Line<'static>>, (u16, u16)) {
    let cont_width = width.max(1) as usize;
    let max_lines = max_lines.max(1) as usize;

    let value = field.get_value();
    let chars: Vec<char> = value.chars().collect();
    let total = chars.len();
    let cursor_idx = field.cursor().1;

    let mut num_lines = if total == 0 {
        1
    } else {
        total.div_ceil(cont_width)
    };
    if total > 0 && cursor_idx == total && total.is_multiple_of(cont_width) {
        num_lines += 1;
    }

    let cursor_line = cursor_idx / cont_width;
    let cursor_col = cursor_idx % cont_width;

    let start_line = if num_lines > max_lines {
        cursor_line
            .saturating_sub(max_lines - 1)
            .min(num_lines - max_lines)
    } else {
        0
    };
    let end_line = (start_line + max_lines).min(num_lines);

    let mut lines = Vec::new();
    for i in start_line..end_line {
        let line_start = (i * cont_width).min(total);
        let line_end = (line_start + cont_width).min(total);
        let line_chars: Vec<char> = chars[line_start..line_end].to_vec();

        let mut text: String = line_chars.into_iter().collect();
        while text.chars().count() < cont_width {
            text.push(' ');
        }
        lines.push(Line::from(Span::raw(text)));
    }

    let cursor_pos = (cursor_col as u16, (cursor_line - start_line) as u16);
    (lines, cursor_pos)
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

pub(crate) fn centered_rect_lines(percent_x: u16, height: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn draw_help(frame: &mut Frame, app: &App) {
    ratatui_which_key::WhichKey::new().render(frame.buffer_mut(), &app.which_key);
}
