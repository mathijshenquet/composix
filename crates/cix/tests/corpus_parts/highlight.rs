use super::super::*;
pub(crate) fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

pub(crate) fn language_for(name: &str) -> Language {
    match name {
        "Cixfile" => Language::Cixfile,
        "Dockerfile" => Language::Dockerfile,
        _ if name.ends_with(".json") => Language::Json,
        _ if name.ends_with(".nix") => Language::Nix,
        _ if name.ends_with(".yaml") || name.ends_with(".yml") => Language::Yaml,
        _ => Language::Plain,
    }
}

pub(crate) fn push_span(output: &mut String, class: &str, source: &str) {
    write!(
        output,
        "<span class=\"{class}\">{}</span>",
        html_escape(source)
    )
    .expect("rendering highlighted span");
}

pub(crate) fn push_styled(output: &mut String, class: Option<&str>, source: &str) {
    if source.is_empty() {
        return;
    }
    if let Some(class) = class {
        push_span(output, class, source);
    } else {
        output.push_str(&html_escape(source));
    }
}

pub(crate) fn highlight_interpolations(source: &str, base_class: Option<&str>) -> String {
    let mut output = String::new();
    let mut emitted = 0;
    let mut search_from = 0;
    while let Some(relative_start) = source[search_from..].find("${") {
        let start = search_from + relative_start;
        if start > 0 && source.as_bytes()[start - 1] == b'$' {
            search_from = start + 2;
            continue;
        }
        let Some(relative_end) = source[start + 2..].find('}') else {
            break;
        };
        let end = start + 2 + relative_end + 1;
        push_styled(&mut output, base_class, &source[emitted..start]);
        push_span(&mut output, "tok-interpolation", &source[start..end]);
        emitted = end;
        search_from = end;
    }
    push_styled(&mut output, base_class, &source[emitted..]);
    output
}

pub(crate) fn directive_line(source: &str, body_class: Option<&str>) -> String {
    let indent_end = source.len() - source.trim_start().len();
    let trimmed = &source[indent_end..];
    let directive_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let mut output = html_escape(&source[..indent_end]);
    push_span(&mut output, "tok-directive", &trimmed[..directive_end]);
    output.push_str(&highlight_interpolations(
        &trimmed[directive_end..],
        body_class,
    ));
    output
}

pub(crate) fn heredoc_marker(source: &str) -> Option<String> {
    let start = source.find("<<")? + 2;
    let marker = source[start..]
        .split_whitespace()
        .next()
        .unwrap_or_default();
    (!marker.is_empty()).then(|| marker.trim_matches(['\'', '"']).to_owned())
}

pub(crate) fn highlight_cixfile(source: &str) -> String {
    let mut output = String::new();
    let mut heredoc = None::<String>;
    for physical_line in source.split_inclusive('\n') {
        let (line, newline) = physical_line
            .strip_suffix('\n')
            .map_or((physical_line, ""), |line| (line, "\n"));
        let trimmed = line.trim_start();
        if let Some(marker) = heredoc.as_deref() {
            if trimmed == marker {
                output.push_str(&directive_line(line, None));
                heredoc = None;
            } else {
                output.push_str(&highlight_interpolations(line, Some("tok-heredoc")));
            }
        } else if trimmed.starts_with('#') {
            let indent_end = line.len() - trimmed.len();
            output.push_str(&html_escape(&line[..indent_end]));
            push_span(&mut output, "tok-comment", trimmed);
        } else if trimmed.is_empty() {
            output.push_str(&html_escape(line));
        } else {
            output.push_str(&directive_line(line, None));
            heredoc = heredoc_marker(line);
        }
        output.push_str(newline);
    }
    output
}

pub(crate) fn quoted_end(source: &str, start: usize, quote: u8) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(bytes.len());
        } else if bytes[cursor] == quote {
            return cursor + 1;
        } else {
            cursor += 1;
        }
    }
    bytes.len()
}

pub(crate) fn highlight_shellish(source: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let bytes = source.as_bytes();
        if matches!(bytes[cursor], b'\'' | b'"') {
            let end = quoted_end(source, cursor, bytes[cursor]);
            push_span(&mut output, "tok-string", &source[cursor..end]);
            cursor = end;
        } else if source[cursor..].starts_with("${") && (cursor == 0 || bytes[cursor - 1] != b'$') {
            let end = source[cursor + 2..]
                .find('}')
                .map_or(source.len(), |end| cursor + 2 + end + 1);
            push_span(&mut output, "tok-interpolation", &source[cursor..end]);
            cursor = end;
        } else {
            let character = source[cursor..].chars().next().expect("source character");
            output.push_str(&html_escape(&character.to_string()));
            cursor += character.len_utf8();
        }
    }
    output
}

pub(crate) fn highlight_dockerfile(source: &str) -> String {
    let mut output = String::new();
    let mut continued = false;
    for physical_line in source.split_inclusive('\n') {
        let (line, newline) = physical_line
            .strip_suffix('\n')
            .map_or((physical_line, ""), |line| (line, "\n"));
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let indent_end = line.len() - trimmed.len();
            output.push_str(&html_escape(&line[..indent_end]));
            push_span(&mut output, "tok-comment", trimmed);
        } else if !continued && !trimmed.is_empty() {
            let indent_end = line.len() - trimmed.len();
            let directive_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            output.push_str(&html_escape(&line[..indent_end]));
            push_span(&mut output, "tok-directive", &trimmed[..directive_end]);
            output.push_str(&highlight_shellish(&trimmed[directive_end..]));
        } else {
            output.push_str(&highlight_shellish(line));
        }
        continued = !trimmed.starts_with('#') && line.trim_end().ends_with('\\');
        output.push_str(newline);
    }
    output
}

pub(crate) fn identifier_end(source: &str, start: usize) -> usize {
    let mut end = start;
    for (offset, character) in source[start..].char_indices() {
        if character.is_alphanumeric() || matches!(character, '_' | '-') {
            end = start + offset + character.len_utf8();
        } else {
            break;
        }
    }
    end
}

pub(crate) fn highlight_nix(source: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "assert", "else", "if", "in", "inherit", "let", "or", "rec", "then", "with",
    ];
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        if source[cursor..].starts_with("/*") {
            let end = source[cursor + 2..]
                .find("*/")
                .map_or(source.len(), |end| cursor + 2 + end + 2);
            push_span(&mut output, "tok-comment", &source[cursor..end]);
            cursor = end;
        } else if source.as_bytes()[cursor] == b'#' {
            let end = source[cursor..]
                .find('\n')
                .map_or(source.len(), |end| cursor + end);
            push_span(&mut output, "tok-comment", &source[cursor..end]);
            cursor = end;
        } else if source.as_bytes()[cursor] == b'"' {
            let end = quoted_end(source, cursor, b'"');
            push_span(&mut output, "tok-string", &source[cursor..end]);
            cursor = end;
        } else if source[cursor..].starts_with("${") {
            let end = source[cursor + 2..]
                .find('}')
                .map_or(source.len(), |end| cursor + 2 + end + 1);
            push_span(&mut output, "tok-interpolation", &source[cursor..end]);
            cursor = end;
        } else {
            let character = source[cursor..].chars().next().expect("source character");
            if character.is_alphabetic() || character == '_' {
                let end = identifier_end(source, cursor);
                let word = &source[cursor..end];
                if KEYWORDS.contains(&word) {
                    push_span(&mut output, "tok-directive", word);
                } else {
                    output.push_str(&html_escape(word));
                }
                cursor = end;
            } else if character.is_ascii_digit() {
                let end = source[cursor..]
                    .find(|value: char| !value.is_ascii_digit() && value != '.')
                    .map_or(source.len(), |end| cursor + end);
                push_span(&mut output, "tok-number", &source[cursor..end]);
                cursor = end;
            } else {
                output.push_str(&html_escape(&character.to_string()));
                cursor += character.len_utf8();
            }
        }
    }
    output
}

pub(crate) fn highlight_json(source: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let character = source[cursor..].chars().next().expect("source character");
        if character == '"' {
            let end = quoted_end(source, cursor, b'"');
            let is_key = source[end..]
                .chars()
                .find(|character| !character.is_whitespace())
                == Some(':');
            push_span(
                &mut output,
                if is_key { "tok-key" } else { "tok-string" },
                &source[cursor..end],
            );
            cursor = end;
        } else if character.is_ascii_digit() || character == '-' {
            let end = source[cursor..]
                .find(|value: char| {
                    !value.is_ascii_digit() && !matches!(value, '-' | '+' | '.' | 'e' | 'E')
                })
                .map_or(source.len(), |end| cursor + end);
            push_span(&mut output, "tok-number", &source[cursor..end]);
            cursor = end;
        } else if character.is_alphabetic() {
            let end = identifier_end(source, cursor);
            push_span(&mut output, "tok-literal", &source[cursor..end]);
            cursor = end;
        } else {
            output.push_str(&html_escape(&character.to_string()));
            cursor += character.len_utf8();
        }
    }
    output
}

pub(crate) fn yaml_comment_start(source: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' && quote == Some('"') {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if character == '#' && quote.is_none() {
            return Some(index);
        }
    }
    None
}

pub(crate) fn yaml_key_end(source: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in source.char_indices() {
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if character == ':' && quote.is_none() {
            return Some(index);
        }
    }
    None
}

pub(crate) fn highlight_yaml_value(source: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let character = source[cursor..].chars().next().expect("source character");
        if matches!(character, '\'' | '"') {
            let end = quoted_end(source, cursor, character as u8);
            push_span(&mut output, "tok-string", &source[cursor..end]);
            cursor = end;
        } else if character.is_ascii_digit() || character == '-' {
            let end = source[cursor..]
                .find(|value: char| !value.is_ascii_digit() && !matches!(value, '-' | '.'))
                .map_or(source.len(), |end| cursor + end);
            push_span(&mut output, "tok-number", &source[cursor..end]);
            cursor = end;
        } else if character.is_alphabetic() {
            let end = identifier_end(source, cursor);
            let word = &source[cursor..end];
            if matches!(word, "false" | "null" | "true" | "yes" | "no") {
                push_span(&mut output, "tok-literal", word);
            } else {
                output.push_str(&html_escape(word));
            }
            cursor = end;
        } else {
            output.push_str(&html_escape(&character.to_string()));
            cursor += character.len_utf8();
        }
    }
    output
}

pub(crate) fn highlight_yaml(source: &str) -> String {
    let mut output = String::new();
    for physical_line in source.split_inclusive('\n') {
        let (line, newline) = physical_line
            .strip_suffix('\n')
            .map_or((physical_line, ""), |line| (line, "\n"));
        let comment_start = yaml_comment_start(line).unwrap_or(line.len());
        let code = &line[..comment_start];
        let indent_end = code.len() - code.trim_start().len();
        let trimmed = &code[indent_end..];
        output.push_str(&html_escape(&code[..indent_end]));
        let key_offset = trimmed.strip_prefix("- ").map_or(0, |_| 2);
        output.push_str(&html_escape(&trimmed[..key_offset]));
        let rest = &trimmed[key_offset..];
        if let Some(key_end) = yaml_key_end(rest) {
            push_span(&mut output, "tok-key", &rest[..key_end]);
            output.push(':');
            output.push_str(&highlight_yaml_value(&rest[key_end + 1..]));
        } else {
            output.push_str(&highlight_yaml_value(rest));
        }
        if comment_start < line.len() {
            push_span(&mut output, "tok-comment", &line[comment_start..]);
        }
        output.push_str(newline);
    }
    output
}

pub(crate) fn highlight(source: &str, language: Language) -> String {
    match language {
        Language::Cixfile => highlight_cixfile(source),
        Language::Dockerfile => highlight_dockerfile(source),
        Language::Json => highlight_json(source),
        Language::Nix => highlight_nix(source),
        Language::Plain => html_escape(source),
        Language::Yaml => highlight_yaml(source),
    }
}
