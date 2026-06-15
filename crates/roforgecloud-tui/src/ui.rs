use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{
    App, EntriesCreateField, MessagingField, OrderedCreateField, OrderedInputField, Screen,
    TextField, SERVICE_ACCOUNT, UNIVERSE_CHOICE_ITEMS,
};
use crate::json_highlight;
use crate::json_tree;

const HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Rgb(60, 60, 60)).fg(Color::White);

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(6)])
        .split(frame.area());

    match app.screen {
        Screen::UniverseChoice => draw_universe_choice(frame, app, chunks[0]),
        Screen::UniverseSelect => draw_universe_select(frame, app, chunks[0]),
        Screen::UniverseInput => draw_universe_input(frame, app, chunks[0]),
        Screen::Menu => draw_menu(frame, app, chunks[0]),
        Screen::Stores => draw_stores(frame, app, chunks[0]),
        Screen::Entries => draw_entries(frame, app, chunks[0]),
        Screen::Value => draw_value(frame, app, chunks[0]),
        Screen::Messaging => draw_messaging(frame, app, chunks[0]),
        Screen::OrderedStoreInput => draw_ordered_store_input(frame, app, chunks[0]),
        Screen::OrderedEntries => draw_ordered_entries(frame, app, chunks[0]),
        Screen::OrderedValue => draw_ordered_value(frame, app, chunks[0]),
    }

    draw_info(frame, app, chunks[1]);

    if app.show_help {
        draw_help(frame, app, frame.area());
    }
}

fn draw_universe_input(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("roforgecloud");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    field_box(frame, rows[0], "Universe ID", &app.universe_input, true);
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .map(|(label, service)| {
            if *service == SERVICE_ACCOUNT {
                if app.logged_in {
                    ListItem::new("Logout")
                } else {
                    ListItem::new("Login")
                }
            } else {
                ListItem::new(*label)
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("roforgecloud"))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    state.select(Some(app.menu_selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_universe_choice(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = UNIVERSE_CHOICE_ITEMS
        .iter()
        .map(|label| ListItem::new(*label))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Universe"))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    state.select(Some(app.universe_choice_selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_universe_select(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_universe_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let id = app.available_universes[i];
            let mut spans = vec![Span::raw(id.to_string())];
            if let Some(name) = app.universe_names.get(&id) {
                spans.push(Span::styled(
                    format!("  ({name})"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = if app.universe_search.value.is_empty() {
        "Select Universe".to_string()
    } else {
        format!("Select Universe (search: {})", app.universe_search.value)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.universe_select_selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_messaging(frame: &mut Frame, app: &App, area: Rect) {
    let universe = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} ({name})", app.universe_id),
        None => app.universe_id.to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Messaging: Publish (universe {universe})"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(inner);

    let topic_active = app.messaging_field == MessagingField::Topic;
    let message_active = app.messaging_field == MessagingField::Message;

    field_box(frame, rows[0], "Topic", &app.messaging_topic, topic_active);
    field_paragraph_box(frame, rows[1], "Message", &app.messaging_message, message_active);
}

fn draw_ordered_store_input(frame: &mut Frame, app: &App, area: Rect) {
    let universe = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} ({name})", app.universe_id),
        None => app.universe_id.to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Ordered Data Stores (universe {universe})"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    let id_active = app.ordered_input_field == OrderedInputField::StoreId;
    let scope_active = app.ordered_input_field == OrderedInputField::Scope;

    field_box(frame, rows[0], "Ordered Data Store ID", &app.ordered_data_store_id, id_active);
    field_box(frame, rows[1], "Scope", &app.ordered_scope, scope_active);
}

fn draw_ordered_entries(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_ordered_entry_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let entry = &app.ordered_entries[i];
            let mut spans = Vec::new();
            if !app.ordered_entries_marked.is_empty() {
                let marker = if app.ordered_entries_marked.contains(&i) {
                    "[x] "
                } else {
                    "[ ] "
                };
                spans.push(Span::raw(marker));
            }
            spans.push(Span::raw(format!("{}  =  {}", entry.id, entry.value)));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let store_label = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!(
            "{} (scope: {}, universe {} ({name}))",
            app.ordered_data_store_id.value, app.ordered_scope.value, app.universe_id
        ),
        None => format!(
            "{} (scope: {})",
            app.ordered_data_store_id.value, app.ordered_scope.value
        ),
    };
    let title = if app.ordered_entries_search.value.is_empty() {
        if app.ordered_entries_marked.is_empty() {
            store_label
        } else {
            format!("{store_label} ({} selected)", app.ordered_entries_marked.len())
        }
    } else if app.ordered_entries_marked.is_empty() {
        format!("{store_label} (search: {})", app.ordered_entries_search.value)
    } else {
        format!(
            "{store_label} (search: {}, {} selected)",
            app.ordered_entries_search.value,
            app.ordered_entries_marked.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.ordered_entries_selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.ordered_create_active {
        draw_ordered_create_popup(frame, app, area);
    }
}

fn draw_ordered_create_popup(frame: &mut Frame, app: &App, area: Rect) {
    let id_active = app.ordered_create_field == OrderedCreateField::Id;
    let value_active = app.ordered_create_field == OrderedCreateField::Value;
    let max_lines = 5;

    let popup = centered_rect_lines(40, max_lines + 2 + 3 + 2, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Create Entry");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    field_box(frame, rows[0], "Id", &app.ordered_create_id, id_active);
    field_paragraph_box(frame, rows[1], "Value", &app.ordered_create_value, value_active);
}

fn draw_ordered_value(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![Line::from("")];

    if app.ordered_value_editing {
        lines.push(Line::from(vec![
            Span::raw("value: "),
            Span::styled(
                format!("{}█", app.ordered_value_edit),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            app.ordered_value.to_string(),
            Style::default().fg(Color::Yellow),
        )));
    }

    if app.ordered_increment_editing {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("increment by: "),
            Span::styled(
                format!("{}█", app.ordered_increment_edit),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.ordered_value_title.clone()),
    );
    frame.render_widget(paragraph, area);
}

fn draw_stores(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .stores
        .iter()
        .enumerate()
        .map(|(i, store)| {
            let mut spans = Vec::new();
            if !app.stores_marked.is_empty() {
                let marker = if app.stores_marked.contains(&i) {
                    "[x] "
                } else {
                    "[ ] "
                };
                spans.push(Span::raw(marker));
            }
            spans.push(Span::raw(store.id.clone()));
            if store.state.as_deref().is_some_and(|state| state != "ACTIVE") {
                spans.push(Span::styled(
                    "  [SCHEDULED DELETION]",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let universe = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} ({name})", app.universe_id),
        None => app.universe_id.to_string(),
    };
    let base_title = format!("Data Stores (universe {universe})");
    let title = if app.stores_marked.is_empty() {
        base_title
    } else {
        format!("{base_title} ({} selected)", app.stores_marked.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !app.stores.is_empty() {
        state.select(Some(app.stores_selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.stores_new_active {
        draw_stores_new_popup(frame, app, area);
    }
}

fn draw_stores_new_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_lines(50, 5, area);

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Create entry in new store");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    field_box(frame, inner, "Data Store ID", &app.stores_new_id, true);
}

fn draw_entries(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_entry_indices();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let entry = &app.entries[i];
            let mut spans = Vec::new();
            if !app.entries_marked.is_empty() {
                let marker = if app.entries_marked.contains(&i) {
                    "[x] "
                } else {
                    "[ ] "
                };
                spans.push(Span::raw(marker));
            }
            spans.push(Span::raw(entry.id.clone()));
            if let Some(username) =
                crate::userlookup::extract_id(&entry.id).and_then(|id| app.usernames.get(&id))
            {
                spans.push(Span::styled(
                    format!("  ({username})"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let store_label = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!(
            "{} (universe {} ({name}))",
            app.data_store_id, app.universe_id
        ),
        None => app.data_store_id.clone(),
    };
    let title = if app.entries_search.value.is_empty() {
        if app.entries_marked.is_empty() {
            store_label
        } else {
            format!("{store_label} ({} selected)", app.entries_marked.len())
        }
    } else if app.entries_marked.is_empty() {
        format!("{store_label} (search: {})", app.entries_search.value)
    } else {
        format!(
            "{store_label} (search: {}, {} selected)",
            app.entries_search.value,
            app.entries_marked.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.entries_selected));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if app.entries_create_active {
        draw_entries_create_popup(frame, app, area);
    }
}

fn draw_entries_create_popup(frame: &mut Frame, app: &App, area: Rect) {
    let id_active = app.entries_create_field == EntriesCreateField::Id;
    let value_active = app.entries_create_field == EntriesCreateField::Value;
    let max_lines = 6;

    let popup = centered_rect_lines(50, max_lines + 2 + 3 + 2, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Create Entry (value: JSON)");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    field_box(frame, rows[0], "Id", &app.entries_create_id, id_active);
    field_paragraph_box(frame, rows[1], "Value", &app.entries_create_value, value_active);
}

fn draw_value(frame: &mut Frame, app: &App, area: Rect) {
    if app.tree_mode {
        draw_tree(frame, app, area);
        return;
    }

    let paragraph = Paragraph::new(json_highlight::highlight(&app.value_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.value_title.clone()),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.value_scroll, 0));

    frame.render_widget(paragraph, area);
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

fn draw_tree(frame: &mut Frame, app: &App, area: Rect) {
    let Some(tree) = &app.value_tree else {
        return;
    };
    let rows = json_tree::flatten(tree);

    let mut edit_cursor_col = 0u16;

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

            if i == app.tree_cursor && app.tree_editing_key {
                let field = &app.tree_edit_key;
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

            if i == app.tree_cursor && app.tree_editing {
                let field = &app.tree_edit_text;
                let prefix_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                let cursor_idx = field.value[..field.cursor].chars().count();
                edit_cursor_col = (prefix_width + cursor_idx) as u16;
                let style = Style::new().bg(Color::Rgb(50, 50, 50)).fg(Color::Yellow);
                spans.push(Span::styled(field.value.clone(), style));
            } else {
                spans.push(Span::styled(
                    row.preview.clone(),
                    scalar_style(&row.preview),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let editing = app.tree_editing || app.tree_editing_key;

    let title = if editing {
        format!("{} (editing)", app.value_title)
    } else {
        format!("{} (tree)", app.value_title)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    if editing {
        state.select(None);
        let visible_height = area.height.saturating_sub(2) as usize;
        let max_offset = rows.len().saturating_sub(visible_height);
        *state.offset_mut() = app
            .tree_cursor
            .saturating_sub(visible_height / 2)
            .min(max_offset);
    } else {
        state.select(Some(app.tree_cursor));
    }
    frame.render_stateful_widget(list, area, &mut state);

    if editing {
        let inner = Block::default().borders(Borders::ALL).inner(area);
        let row_y = app.tree_cursor.saturating_sub(state.offset());
        if row_y < inner.height as usize {
            frame.set_cursor_position((inner.x + edit_cursor_col, inner.y + row_y as u16));
        }
    }
}

fn draw_info(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
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

const MOVE: &str = "↑/↓ or j/k: move";
const SCROLL: &str = "↑/↓ or j/k: scroll";
const QUIT: &str = "q: quit";
const BACK_QUIT: &str = "esc/h: back   q: quit";

fn screen_binds(app: &App) -> String {
    match app.screen {
        Screen::Menu => format!("{MOVE}   {}   {QUIT}", crate::menu_hints(app)),
        Screen::UniverseChoice => {
            format!("{MOVE}   {}   {BACK_QUIT}", crate::universe_choice_hints(app))
        }
        Screen::UniverseSelect if app.universe_search_active => {
            "type to search by id or name   enter/esc: confirm".to_string()
        }
        Screen::UniverseSelect => {
            format!("{MOVE}   {}   {BACK_QUIT}", crate::universe_select_hints(app))
        }
        Screen::UniverseInput => "type a universe id   enter: confirm   esc: back".to_string(),
        Screen::Stores if app.stores_new_active => {
            "type a data store id   enter: continue   esc: cancel".to_string()
        }
        Screen::Stores => format!("{MOVE}   {}   {BACK_QUIT}", crate::stores_hints(app)),
        Screen::Entries if app.entries_search_active => {
            "type to search by id or username   enter/esc: confirm".to_string()
        }
        Screen::Entries if app.entries_create_choosing => {
            "n: form   e: $EDITOR   esc: cancel".to_string()
        }
        Screen::Entries if app.entries_create_active => {
            "tab: switch field   enter: create   esc: cancel".to_string()
        }
        Screen::Entries => format!("{MOVE}   {}   {BACK_QUIT}", crate::entries_hints(app)),
        Screen::Value if app.tree_mode && (app.tree_editing || app.tree_editing_key) => {
            "type to edit   enter: confirm   esc: cancel".to_string()
        }
        Screen::Value if app.tree_mode => {
            format!("{MOVE}   {}", crate::tree_hints(app))
        }
        Screen::Value => format!(
            "{SCROLL}   pgup/pgdn: scroll x10   {}   {BACK_QUIT}",
            crate::value_hints(app)
        ),
        Screen::Messaging => "tab: switch field   enter: publish   esc: back".to_string(),
        Screen::OrderedStoreInput => "tab: switch field   enter: confirm   esc: back".to_string(),
        Screen::OrderedEntries if app.ordered_entries_search_active => {
            "type to search by id   enter/esc: confirm".to_string()
        }
        Screen::OrderedEntries if app.ordered_create_choosing => {
            "n: form   e: $EDITOR   esc: cancel".to_string()
        }
        Screen::OrderedEntries if app.ordered_create_active => {
            "tab: switch field   enter: create   esc: cancel".to_string()
        }
        Screen::OrderedEntries => {
            format!("{MOVE}   {}   {BACK_QUIT}", crate::ordered_entries_hints(app))
        }
        Screen::OrderedValue if app.ordered_value_editing => {
            "type to edit   enter: confirm   esc: cancel".to_string()
        }
        Screen::OrderedValue if app.ordered_increment_editing => {
            "type amount   enter: confirm   esc: cancel".to_string()
        }
        Screen::OrderedValue => format!("{}   {BACK_QUIT}", crate::ordered_value_hints(app)),
    }
}

fn draw_keybinds(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(pending) = &app.pending_confirm {
        let paragraph = Paragraph::new(Line::from(pending.footer_hint()))
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
        return;
    }

    if app.tree_pending_leader {
        let paragraph = Paragraph::new(Line::from(
            "v: edit value   k: edit key   e: edit in $EDITOR   any other key: cancel",
        ))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
        return;
    }

    let binds = screen_binds(app);
    let binds = if app.text_input_active() {
        binds
    } else {
        format!("{binds}   ?: help")
    };
    let paragraph = Paragraph::new(Line::from(binds))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn parse_hints(hints: &str) -> Vec<(String, String)> {
    hints
        .split("   ")
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let (key, desc) = entry.split_once(": ")?;
            Some((key.to_string(), desc.to_string()))
        })
        .collect()
}

type HintList = Vec<(String, String)>;

fn help_sections(app: &App) -> Vec<(&'static str, HintList)> {
    let (movement, other): (HintList, HintList) = parse_hints(&screen_binds(app))
        .into_iter()
        .partition(|(key, _)| key.contains('↑') || key.starts_with("pgup"));

    let mut sections = Vec::new();
    if !movement.is_empty() {
        sections.push(("Movement", movement));
    }
    if !other.is_empty() {
        sections.push(("This screen", other));
    }
    sections.push(("General", parse_hints("?: toggle this help")));
    sections
}

fn field_box(frame: &mut Frame, area: Rect, title: &str, field: &TextField, active: bool) {
    let block = Block::default().borders(Borders::ALL).title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (line, cursor_col) = field_line("", field, active, inner.width);
    frame.render_widget(Paragraph::new(line), inner);
    if active {
        frame.set_cursor_position((inner.x + cursor_col, inner.y));
    }
}

fn field_paragraph_box(frame: &mut Frame, area: Rect, title: &str, field: &TextField, active: bool) {
    let block = Block::default().borders(Borders::ALL).title(title.to_string());
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

    let mut num_lines = if total == 0 { 1 } else { total.div_ceil(cont_width) };
    if total > 0 && cursor_idx == total && total.is_multiple_of(cont_width) {
        num_lines += 1;
    }

    let cursor_line = cursor_idx / cont_width;
    let cursor_col = cursor_idx % cont_width;

    let start_line = if num_lines > max_lines {
        cursor_line.saturating_sub(max_lines - 1).min(num_lines - max_lines)
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

fn centered_rect_lines(percent_x: u16, height: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect { x, y, width, height }
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 60, area);

    let mut lines = Vec::new();
    for (title, binds) in help_sections(app) {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));

        let key_width = binds.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
        for (key, desc) in binds {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {key:<key_width$}  "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(desc),
            ]));
        }
    }

    frame.render_widget(Clear, popup);
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Keybinds (? or esc to close)"),
    );
    frame.render_widget(paragraph, popup);
}
