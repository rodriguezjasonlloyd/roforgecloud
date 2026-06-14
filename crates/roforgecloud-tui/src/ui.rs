use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, MessagingField, Screen, SERVICE_ACCOUNT, UNIVERSE_CHOICE_ITEMS};
use crate::json_highlight;
use crate::json_tree;

const HIGHLIGHT_STYLE: Style = Style::new()
    .bg(Color::Cyan)
    .fg(Color::Black)
    .add_modifier(Modifier::BOLD);

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
    }

    draw_info(frame, app, chunks[1]);
}

fn draw_universe_input(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!("Universe ID: {}", app.universe_input);
    let paragraph = Paragraph::new(Line::from(text))
        .block(Block::default().borders(Borders::ALL).title("roforgecloud"));
    frame.render_widget(paragraph, area);
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

    let title = if app.universe_search.is_empty() {
        "Select Universe".to_string()
    } else {
        format!("Select Universe (search: {})", app.universe_search)
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
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let topic_style = match app.messaging_field {
        MessagingField::Topic => HIGHLIGHT_STYLE,
        MessagingField::Message => Style::default(),
    };
    let message_style = match app.messaging_field {
        MessagingField::Message => HIGHLIGHT_STYLE,
        MessagingField::Topic => Style::default(),
    };

    frame.render_widget(
        Paragraph::new(Line::from(format!("Topic:   {}", app.messaging_topic))).style(topic_style),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(format!("Message: {}", app.messaging_message)))
            .style(message_style),
        rows[1],
    );
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
            if let Some(state) = &store.state {
                if state != "ACTIVE" {
                    spans.push(Span::styled(
                        format!("  [{state}]"),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let universe = match app.universe_names.get(&app.universe_id) {
        Some(name) => format!("{} ({name})", app.universe_id),
        None => app.universe_id.to_string(),
    };
    let base_title = if app.stores_show_deleted {
        format!("Data Stores (universe {universe}, incl. deleted)")
    } else {
        format!("Data Stores (universe {universe})")
    };
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
            "{} (universe {} / {name})",
            app.data_store_id, app.universe_id
        ),
        None => app.data_store_id.clone(),
    };
    let title = if app.entries_search.is_empty() {
        if app.entries_marked.is_empty() {
            store_label
        } else {
            format!("{store_label} ({} selected)", app.entries_marked.len())
        }
    } else if app.entries_marked.is_empty() {
        format!("{store_label} (search: {})", app.entries_search)
    } else {
        format!(
            "{store_label} (search: {}, {} selected)",
            app.entries_search,
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

            if let Some(key) = &row.key {
                spans.push(Span::styled(
                    format!("{key:?}: "),
                    Style::default().fg(Color::Cyan),
                ));
            }

            if i == app.tree_cursor && app.tree_editing {
                spans.push(Span::styled(
                    format!("{}█", app.tree_edit_text),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            } else {
                spans.push(Span::styled(
                    row.preview.clone(),
                    scalar_style(&row.preview),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = if app.tree_editing {
        format!("{} (editing)", app.value_title)
    } else {
        format!("{} (tree)", app.value_title)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(HIGHLIGHT_STYLE);

    let mut state = ListState::default();
    state.select(Some(app.tree_cursor));
    frame.render_stateful_widget(list, area, &mut state);
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

fn draw_keybinds(frame: &mut Frame, app: &App, area: Rect) {
    let binds = match app.screen {
        Screen::Menu => "↑/↓ or j/k: move   enter/l: open   q: quit",
        Screen::UniverseChoice => "↑/↓ or j/k: move   enter/l: select   esc/h: back   q: quit",
        Screen::UniverseSelect if app.universe_search_active => {
            "type to search by id or name   enter/esc: confirm"
        }
        Screen::UniverseSelect => {
            "↑/↓ or j/k: move   enter/l: select   /: search   esc/h: back   q: quit"
        }
        Screen::UniverseInput => "type a universe id   enter: confirm   esc: back",
        Screen::Stores => "↑/↓ or j/k: move   enter/l: open   space: select   a: select all   r: refresh   d d: delete (selected)   u u: undelete (selected)   D: toggle deleted   esc/h: back   q: quit",
        Screen::Entries if app.entries_search_active => {
            "type to search by id or username   enter/esc: confirm"
        }
        Screen::Entries => "↑/↓ or j/k: move   enter/l: view   space: select   a: select all   n/p: next/prev page   r: refresh   /: search   d d: delete (selected)   esc/h: back   q: quit",
        Screen::Value if app.tree_mode && app.tree_editing => {
            "type to edit   enter: confirm   esc: cancel"
        }
        Screen::Value if app.tree_mode => {
            "↑/↓ or j/k: move   space: fold/unfold   enter: edit value   ctrl+s: save   d d: delete entry   esc: exit tree   q: quit"
        }
        Screen::Value => {
            "↑/↓ or j/k: scroll   pgup/pgdn: scroll x10   r: refresh   enter/l: tree edit   e: edit in $EDITOR   d d: delete   esc/h: back   q: quit"
        }
        Screen::Messaging => "tab: switch field   enter: publish   esc: back",
    };
    let paragraph = Paragraph::new(Line::from(binds))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
