use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Screen, TextFieldExt, TreeTarget};
use crate::json_tree;
use crate::screens;
use crate::status;
use crate::update;

pub(crate) const HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Rgb(60, 60, 60)).fg(Color::White);

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let keybinds_height = keybinds_height(app, area.width);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3 + keybinds_height)])
        .split(area);

    (screens::def(app.screen).draw)(frame, app, chunks[0]);

    draw_info(frame, app, chunks[1], keybinds_height);

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

    let title_base = match app.tree_target {
        TreeTarget::Value => app.value.title.clone(),
        TreeTarget::EntriesCreate | TreeTarget::MemoryCreate => "Value".to_string(),
    };
    let title = if editing {
        format!("{title_base} (editing)")
    } else {
        format!("{title_base} (tree)")
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

fn draw_info(frame: &mut Frame, app: &App, area: Rect, keybinds_height: u16) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(keybinds_height)])
        .split(area);

    draw_status(frame, app, rows[0]);
    draw_keybinds(frame, app, rows[1]);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.loading {
        status::LOADING.to_string()
    } else {
        app.status.clone()
    };
    let paragraph = Paragraph::new(Line::from(text)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

use update::{join_hints, hint_bar_entries, InputHint, Scope, BACK_QUIT, MOVE, QUIT, SCROLL};

fn screen_binds(app: &App) -> String {
    match app.screen {
        Screen::Menu => join_hints(&[MOVE, &hint_bar_entries(app, Scope::Menu), QUIT]),
        Screen::UniverseChoice => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::UniverseChoice), BACK_QUIT])
        }
        Screen::UniverseSelect if app.universe_select.search_active => {
            InputHint::SearchByIdOrName.to_string()
        }
        Screen::UniverseSelect => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::UniverseSelect), BACK_QUIT])
        }
        Screen::UniverseInput => InputHint::UniverseInput.to_string(),
        Screen::Stores if app.stores.new_active => InputHint::StoreInput.to_string(),
        Screen::Stores => join_hints(&[MOVE, &hint_bar_entries(app, Scope::Stores), BACK_QUIT]),
        Screen::Entries if app.entries.search_active => {
            InputHint::SearchByIdOrUsername.to_string()
        }
        Screen::Entries if app.tree_editor.as_ref().is_some_and(|t| t.is_editing()) => {
            InputHint::EditText.to_string()
        }
        Screen::Entries if app.tree_editor.is_some() => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::Tree)])
        }
        Screen::Entries if app.entries.create_choosing => InputHint::CreateChoosing.to_string(),
        Screen::Entries if app.entries.create_active => update::entries_create_hints(app),
        Screen::Entries => join_hints(&[MOVE, &hint_bar_entries(app, Scope::Entries), BACK_QUIT]),
        Screen::Value if app.tree_editor.as_ref().is_some_and(|t| t.is_editing()) => {
            InputHint::EditText.to_string()
        }
        Screen::Value if app.tree_editor.is_some() => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::Tree)])
        }
        Screen::Value if app.memory_entries.ttl_editing => InputHint::TtlEdit.to_string(),
        Screen::Value => join_hints(&[SCROLL, &hint_bar_entries(app, Scope::Value), BACK_QUIT]),
        Screen::Messaging => InputHint::Messaging.to_string(),
        Screen::OrderedStoreInput => InputHint::OrderedStoreInput.to_string(),
        Screen::OrderedEntries if app.ordered_entries.search_active => {
            InputHint::SearchById.to_string()
        }
        Screen::OrderedEntries if app.ordered_entries.create_choosing => {
            InputHint::CreateChoosing.to_string()
        }
        Screen::OrderedEntries if app.ordered_entries.create_active => {
            InputHint::OrderedCreateActive.to_string()
        }
        Screen::OrderedEntries => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::OrderedEntries), BACK_QUIT])
        }
        Screen::OrderedValue if app.ordered_value.editing => InputHint::EditText.to_string(),
        Screen::OrderedValue if app.ordered_value.increment_editing => {
            InputHint::AmountEdit.to_string()
        }
        Screen::OrderedValue => {
            join_hints(&[&hint_bar_entries(app, Scope::OrderedValue), BACK_QUIT])
        }
        Screen::MemoryStoreInput => InputHint::MemoryStoreInput.to_string(),
        Screen::MemoryStoreEntries if app.memory_entries.search_active => {
            InputHint::SearchById.to_string()
        }
        Screen::MemoryStoreEntries
            if app.tree_editor.as_ref().is_some_and(|t| t.is_editing()) =>
        {
            InputHint::EditText.to_string()
        }
        Screen::MemoryStoreEntries if app.tree_editor.is_some() => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::Tree)])
        }
        Screen::MemoryStoreEntries if app.memory_entries.create_choosing => {
            InputHint::CreateChoosing.to_string()
        }
        Screen::MemoryStoreEntries if app.memory_entries.create_active => {
            update::memory_create_hints(app)
        }
        Screen::MemoryStoreEntries if app.memory_entries.ttl_editing => InputHint::TtlEdit.to_string(),
        Screen::MemoryStoreEntries => {
            join_hints(&[MOVE, &hint_bar_entries(app, Scope::MemoryEntries), BACK_QUIT])
        }
    }
}

fn keybinds_text(app: &App) -> String {
    if let Some(pending) = &app.pending_confirm {
        return pending.footer_hint();
    }

    if app.tree_editor.as_ref().is_some_and(|t| t.pending_leader()) {
        return update::InputHint::TreeLeaderMenu.to_string();
    }

    let binds = screen_binds(app);
    if app.text_input_active() {
        binds
    } else {
        format!("{binds}   ?: help")
    }
}

fn keybinds_height(app: &App, width: u16) -> u16 {
    let text = keybinds_text(app);
    let inner_width = width.saturating_sub(2).max(1) as usize;

    let mut lines = 1u16;
    let mut col = 0usize;
    for word in text.split(' ') {
        let word_len = word.chars().count();
        if col == 0 {
            col = word_len;
        } else if col + 1 + word_len > inner_width {
            lines += 1;
            col = word_len;
        } else {
            col += 1 + word_len;
        }
    }

    lines + 2
}

fn draw_keybinds(frame: &mut Frame, app: &App, area: Rect) {
    let paragraph = Paragraph::new(Line::from(keybinds_text(app)))
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: true })
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
