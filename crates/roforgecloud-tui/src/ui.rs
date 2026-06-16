use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{
    App, EntriesCreateField, MemoryCreateField, MessagingField, OrderedCreateField,
    OrderedInputField, Screen, TextField, TreeTarget,
};
use crate::json_highlight;
use crate::json_tree;

pub(crate) const HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Rgb(60, 60, 60)).fg(Color::White);

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let keybinds_height = keybinds_height(app, area.width);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3 + keybinds_height)])
        .split(area);

    (crate::screens::def(app.screen).draw)(frame, app, chunks[0]);

    draw_info(frame, app, chunks[1], keybinds_height);

    if app.which_key.active {
        draw_help(frame, app);
    }
}

pub(crate) fn draw_universe_input(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::universe_input::draw(frame, app, area);
}

pub(crate) fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::menu::draw(frame, app, area);
}

pub(crate) fn draw_universe_choice(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::universe_choice::draw(frame, app, area);
}

pub(crate) fn draw_universe_select(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::universe_select::draw(frame, app, area);
}

pub(crate) fn draw_messaging(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::messaging::draw(frame, app, area);
}

pub(crate) fn draw_ordered_store_input(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::ordered_store_input::draw(frame, app, area);
}

pub(crate) fn draw_ordered_entries(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::ordered_entries::draw(frame, app, area);
}

pub(crate) fn draw_ordered_value(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::ordered_value::draw(frame, app, area);
}

pub(crate) fn draw_memory_store_input(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::memory_store_input::draw(frame, app, area);
}

pub(crate) fn draw_memory_entries(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::memory_entries::draw(frame, app, area);
}

pub(crate) fn draw_stores(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::stores::draw(frame, app, area);
}

pub(crate) fn draw_entries(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::entries::draw(frame, app, area);
}

pub(crate) fn draw_value(frame: &mut Frame, app: &App, area: Rect) {
    crate::screens::value::draw(frame, app, area);
}

fn scalar_style(preview: &str) -> Style {
    if preview.starts_with('"') {
        Style::default().fg(Color::Green)
    } else if preview == "true" || preview == "false" {
        Style::default().fg(Color::Magenta)
    } else if preview == "null" {
        Style::default().fg(Color::DarkGray)
    } else if preview.parse::<f64>().is_ok() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
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
                let marker = if row.preview.contains('…') {
                    "▸ "
                } else {
                    "▾ "
                };
                spans.push(Span::styled(marker, Style::default().fg(Color::DarkGray)));
            } else {
                spans.push(Span::raw("  "));
            }

            if i == editor.cursor() && editor.editing_key() {
                let field = editor.edit_key();
                let prefix_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                let cursor_idx = field.value[..field.cursor].chars().count();
                edit_cursor_col = (prefix_width + 1 + cursor_idx) as u16;
                let style = Style::new().bg(Color::Rgb(50, 50, 50)).fg(Color::Yellow);
                spans.push(Span::raw("\""));
                spans.push(Span::styled(field.value.clone(), style));
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
                spans.push(Span::styled(
                    row.preview.clone(),
                    scalar_style(&row.preview),
                ));
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
        "Loading...".to_string()
    } else {
        app.status.clone()
    };
    let paragraph = Paragraph::new(Line::from(text)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

use crate::update::{join_hints, hint_bar_entries, InputHint, Scope, BACK_QUIT, MOVE, QUIT, SCROLL};

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
        Screen::Entries if app.entries.create_active => crate::update::entries_create_hints(app),
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
            crate::update::memory_create_hints(app)
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
        return crate::update::InputHint::TreeLeaderMenu.to_string();
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

pub(crate) fn field_box(frame: &mut Frame, area: Rect, title: &str, field: &TextField, active: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (line, cursor_col) = field_line("", field, active, inner.width);
    frame.render_widget(Paragraph::new(line), inner);
    if active {
        frame.set_cursor_position((inner.x + cursor_col, inner.y));
    }
}

pub(crate) fn field_paragraph_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    field: &TextField,
    active: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (lines, cursor) = field_paragraph("", field, active, inner.width, inner.height);
    frame.render_widget(Paragraph::new(lines), inner);
    if active {
        frame.set_cursor_position((inner.x + cursor.0, inner.y + cursor.1));
    }
}

fn field_line(label: &str, field: &TextField, _active: bool, width: u16) -> (Line<'static>, u16) {
    let label_width = label.chars().count() as u16;
    let field_width = width.saturating_sub(label_width).max(1) as usize;

    let chars: Vec<char> = field.value.chars().collect();
    let cursor_idx = field.value[..field.cursor].chars().count();

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
    let line = Line::from(vec![Span::raw(label.to_string()), Span::raw(text)]);
    let cursor_col = label_width + (cursor_idx - start) as u16;
    (line, cursor_col)
}

fn field_paragraph(
    label: &str,
    field: &TextField,
    _active: bool,
    width: u16,
    max_lines: u16,
) -> (Vec<Line<'static>>, (u16, u16)) {
    let label_width = label.chars().count() as u16;
    let cont_width = width.saturating_sub(label_width).max(1) as usize;
    let max_lines = max_lines.max(1) as usize;

    let chars: Vec<char> = field.value.chars().collect();
    let total = chars.len();
    let cursor_idx = field.value[..field.cursor].chars().count();

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

        let prefix = if i == 0 {
            Span::raw(label.to_string())
        } else {
            Span::raw(" ".repeat(label_width as usize))
        };

        let mut text: String = line_chars.into_iter().collect();
        while text.chars().count() < cont_width {
            text.push(' ');
        }
        lines.push(Line::from(vec![prefix, Span::raw(text)]));
    }

    let cursor_pos = (
        label_width + cursor_col as u16,
        (cursor_line - start_line) as u16,
    );
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
