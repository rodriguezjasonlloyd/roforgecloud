use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};

pub fn highlight(json: &str) -> Text<'static> {
    Text::from(json.lines().map(highlight_line).collect::<Vec<_>>())
}

fn highlight_line(line: &str) -> Line<'static> {
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);

    let mut spans = vec![Span::raw(indent.to_string())];

    if let Some(colon_idx) = key_end(rest) {
        let key = &rest[..colon_idx];
        let after = &rest[colon_idx + 1..];
        let value_start = after.len() - after.trim_start().len();
        let (ws, value) = after.split_at(value_start);

        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::raw(":".to_string()));
        spans.push(Span::raw(ws.to_string()));
        spans.extend(highlight_value(value));
    } else {
        spans.extend(highlight_value(rest));
    }

    Line::from(spans)
}

fn key_end(rest: &str) -> Option<usize> {
    if !rest.starts_with('"') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => {
                i += 1;
                break;
            }
            _ => i += 1,
        }
    }

    let after_quote = &rest[i..];
    let colon_offset = after_quote.find(':')?;
    if after_quote[..colon_offset].trim().is_empty() {
        Some(i + colon_offset)
    } else {
        None
    }
}

fn highlight_value(value: &str) -> Vec<Span<'static>> {
    if value.is_empty() {
        return Vec::new();
    }

    let (body, trailing) = match value.strip_suffix(',') {
        Some(body) => (body, ","),
        None => (value, ""),
    };

    let color = match body.chars().next().unwrap() {
        '"' => Some(Color::Green),
        't' | 'f' => Some(Color::Magenta),
        'n' => Some(Color::DarkGray),
        '-' | '0'..='9' => Some(Color::Yellow),
        _ => None,
    };

    match color {
        Some(color) => vec![
            Span::styled(body.to_string(), Style::default().fg(color)),
            Span::raw(trailing.to_string()),
        ],
        None => vec![Span::raw(value.to_string())],
    }
}
