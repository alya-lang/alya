use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Function,
    If,
    While,
    For,
    Repeat,
    Try,
    Struct,
    Enum,
    When,
    WhenArm,
    Brace,
    Bracket,
    Extern,
}

fn is_in_extern_block(stack: &[BlockKind]) -> bool {
    for b in stack.iter().rev() {
        match b {
            BlockKind::Brace | BlockKind::Bracket => continue,
            BlockKind::Extern => return true,
            _ => return false,
        }
    }
    false
}

fn pop_leading_closing_delimiters(code: &str, block_stack: &mut Vec<BlockKind>) -> usize {
    let mut popped_bytes = 0;
    let bytes = code.as_bytes();
    while popped_bytes < bytes.len() {
        let b = bytes[popped_bytes];
        let matches_brace = b == b'}' && block_stack.last() == Some(&BlockKind::Brace);
        let matches_bracket = b == b']' && block_stack.last() == Some(&BlockKind::Bracket);
        if matches_brace || matches_bracket {
            block_stack.pop();
            popped_bytes += 1;
        } else {
            break;
        }
    }
    popped_bytes
}

fn scan_delimiters_after_leading(code_slice: &str, block_stack: &mut Vec<BlockKind>) {
    let mut in_str = false;
    let mut quote_char = '"';
    let mut escaped = false;
    let bytes = code_slice.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote_char as u8 {
                in_str = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' || b == b'`' {
            in_str = true;
            quote_char = b as char;
            i += 1;
            continue;
        }

        if b == b'{' {
            block_stack.push(BlockKind::Brace);
        } else if b == b'[' {
            block_stack.push(BlockKind::Bracket);
        } else if (b == b'}' && block_stack.last() == Some(&BlockKind::Brace))
            || (b == b']' && block_stack.last() == Some(&BlockKind::Bracket))
        {
            block_stack.pop();
        }

        i += 1;
    }
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut quote = '"';
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote as u8 {
                in_str = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' || b == b'`' {
            in_str = true;
            quote = b as char;
            i += 1;
            continue;
        }

        // Single-line comments starting with '#' or '//'
        if b == b'#' {
            return line[..i].trim();
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            return line[..i].trim();
        }

        i += 1;
    }
    line.trim()
}

fn find_word_outside_quotes(s: &str, word: &str) -> Option<usize> {
    let mut in_str = false;
    let mut quote = '"';
    let mut escaped = false;
    let bytes = s.as_bytes();
    let word_bytes = word.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote as u8 {
                in_str = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' || b == b'`' {
            in_str = true;
            quote = b as char;
            i += 1;
            continue;
        }

        if i + word_bytes.len() <= bytes.len() && &bytes[i..i + word_bytes.len()] == word_bytes {
            let before_ok = i == 0
                || bytes[i - 1].is_ascii_whitespace()
                || bytes[i - 1] == b'('
                || bytes[i - 1] == b')';
            let after_idx = i + word_bytes.len();
            let after_ok = after_idx == bytes.len()
                || bytes[after_idx].is_ascii_whitespace()
                || bytes[after_idx] == b'('
                || bytes[after_idx] == b')';
            if before_ok && after_ok {
                return Some(i);
            }
        }

        i += 1;
    }
    None
}

fn find_str_outside_quotes(s: &str, needle: &str) -> Option<usize> {
    let mut in_str = false;
    let mut quote = '"';
    let mut escaped = false;
    let bytes = s.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote as u8 {
                in_str = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' || b == b'`' {
            in_str = true;
            quote = b as char;
            i += 1;
            continue;
        }

        if i + needle_bytes.len() <= bytes.len()
            && &bytes[i..i + needle_bytes.len()] == needle_bytes
        {
            return Some(i);
        }

        i += 1;
    }
    None
}

fn has_word_outside_quotes(s: &str, word: &str) -> bool {
    find_word_outside_quotes(s, word).is_some()
}

fn has_inline_if(code: &str) -> bool {
    has_word_outside_quotes(code, "then") && has_word_outside_quotes(code, "else")
}

fn is_when_arm_inline(code: &str) -> bool {
    if let Some(pos) = find_word_outside_quotes(code, "then") {
        let after = code[pos + "then".len()..].trim();
        !after.is_empty()
    } else if let Some(pos) = find_str_outside_quotes(code, "=>") {
        let after = code[pos + "=>".len()..].trim();
        !after.is_empty()
    } else {
        false
    }
}

fn is_when_else_inline(code: &str) -> bool {
    let trimmed = code.trim();
    if trimmed == "else" {
        return false;
    }
    if let Some(pos) = find_word_outside_quotes(code, "else") {
        let rest = code[pos + "else".len()..].trim();
        if rest.is_empty() {
            return false;
        }
        if rest == "then" {
            return false;
        }
        if let Some(then_pos) = find_word_outside_quotes(rest, "then") {
            let after_then = rest[then_pos + "then".len()..].trim();
            !after_then.is_empty()
        } else if let Some(arrow_pos) = find_str_outside_quotes(rest, "=>") {
            let after_arrow = rest[arrow_pos + "=>".len()..].trim();
            !after_arrow.is_empty()
        } else {
            !rest.is_empty()
        }
    } else {
        false
    }
}

fn ends_with_word_outside_quotes(s: &str, word: &str) -> bool {
    let bytes = s.as_bytes();
    let word_bytes = word.as_bytes();
    let mut i = bytes.len();

    // Skip trailing whitespace from the end
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }

    if i < word_bytes.len() {
        return false;
    }

    let start = i - word_bytes.len();
    if &bytes[start..i] != word_bytes {
        return false;
    }

    // Check boundary before the word (it must be whitespace, ')', ';', etc.)
    if start > 0 {
        let b_before = bytes[start - 1];
        if !b_before.is_ascii_whitespace() && b_before != b')' && b_before != b';' {
            return false;
        }
    }

    // Now verify that `start..i` is outside quotes
    let mut in_str_scan = false;
    let mut q_scan = '"';
    let mut esc_scan = false;
    let mut j = 0;
    while j < start {
        let b = bytes[j];
        if in_str_scan {
            if esc_scan {
                esc_scan = false;
            } else if b == b'\\' {
                esc_scan = true;
            } else if b == q_scan as u8 {
                in_str_scan = false;
            }
            j += 1;
            continue;
        }

        if b == b'"' || b == b'`' {
            in_str_scan = true;
            q_scan = b as char;
            j += 1;
            continue;
        }

        j += 1;
    }

    !in_str_scan
}

fn get_block_starter(code: &str, in_extern: bool) -> Option<BlockKind> {
    // If the line ends with 'end' outside quotes, whatever block it opened is immediately closed on the same line
    if ends_with_word_outside_quotes(code, "end") {
        return None;
    }

    let code_trimmed = code.trim();
    let code_after_pub = if let Some(stripped) = code_trimmed.strip_prefix("pub ") {
        stripped.trim_start()
    } else {
        code_trimmed
    };

    let first_word = code_after_pub.split_whitespace().next().unwrap_or("");
    if first_word == "extern" {
        return Some(BlockKind::Extern);
    }
    if !in_extern {
        if first_word == "function"
            || first_word == "fn"
            || code_after_pub.starts_with("function(")
            || code_after_pub.starts_with("fn(")
        {
            return Some(BlockKind::Function);
        }
        if (has_word_outside_quotes(code_after_pub, "fn")
            || has_word_outside_quotes(code_after_pub, "function"))
            && !code_after_pub.contains("=>")
            && (code_after_pub.contains("fn(")
                || code_after_pub.contains("fn (")
                || code_after_pub.contains("function(")
                || code_after_pub.contains("function ("))
        {
            return Some(BlockKind::Function);
        }
    }
    if first_word == "if" || code_after_pub.starts_with("if(") {
        if has_inline_if(code_after_pub) {
            return None;
        }
        return Some(BlockKind::If);
    }
    if first_word == "while" || code_after_pub.starts_with("while(") {
        return Some(BlockKind::While);
    }
    if first_word == "for" {
        return Some(BlockKind::For);
    }
    if first_word == "repeat" {
        return Some(BlockKind::Repeat);
    }
    if first_word == "try" {
        return Some(BlockKind::Try);
    }
    if first_word == "when"
        || code_after_pub.starts_with("when(")
        || (has_word_outside_quotes(code_after_pub, "when")
            && !ends_with_word_outside_quotes(code_after_pub, "end"))
    {
        return Some(BlockKind::When);
    }
    if first_word == "struct" {
        return Some(BlockKind::Struct);
    }
    if first_word == "enum" {
        return Some(BlockKind::Enum);
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MultilineLiteralState {
    TripleQuote,
    RawString,
    DoubleQuote,
    Comment,
}

fn scan_line_multiline_state(
    line: &str,
    mut state: Option<MultilineLiteralState>,
) -> Option<MultilineLiteralState> {
    let bytes = line.as_bytes();
    let mut escaped = false;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        match state {
            None => {
                if b == b'#' {
                    break;
                }
                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    break;
                }
                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    state = Some(MultilineLiteralState::Comment);
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    if i + 2 < bytes.len() && bytes[i + 1] == b'"' && bytes[i + 2] == b'"' {
                        state = Some(MultilineLiteralState::TripleQuote);
                        i += 3;
                        continue;
                    } else {
                        state = Some(MultilineLiteralState::DoubleQuote);
                        escaped = false;
                        i += 1;
                        continue;
                    }
                }
                if b == b'`' {
                    state = Some(MultilineLiteralState::RawString);
                    i += 1;
                    continue;
                }
                i += 1;
            }
            Some(MultilineLiteralState::DoubleQuote) => {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    state = None;
                }
                i += 1;
            }
            Some(MultilineLiteralState::TripleQuote) => {
                if b == b'"' && i + 2 < bytes.len() && bytes[i + 1] == b'"' && bytes[i + 2] == b'"'
                {
                    state = None;
                    i += 3;
                    continue;
                }
                i += 1;
            }
            Some(MultilineLiteralState::RawString) => {
                if b == b'`' {
                    state = None;
                }
                i += 1;
            }
            Some(MultilineLiteralState::Comment) => {
                if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    state = None;
                    i += 2;
                    continue;
                }
                i += 1;
            }
        }
    }

    state
}

fn is_comment_group_preceded_by_blank(lines: &[&str], current_idx: usize) -> bool {
    let mut i = current_idx;
    while i > 0 {
        i -= 1;
        let t = lines[i].trim();
        if t.is_empty() {
            return true;
        }
        let c = strip_line_comment(t);
        if !c.is_empty() {
            return false;
        }
    }
    true
}

fn determine_comment_indent(
    lines: &[&str],
    current_idx: usize,
    raw_line: &str,
    preceded_by_blank: bool,
    block_stack: &[BlockKind],
) -> usize {
    let default_indent = block_stack.len();
    if default_indent == 0 {
        return 0;
    }

    // Look ahead for the next non-empty, non-comment code line
    let mut next_code = None;
    for &future_line in &lines[current_idx + 1..] {
        let f_trimmed = future_line.trim();
        if f_trimmed.is_empty() {
            continue;
        }
        let f_code = strip_line_comment(f_trimmed);
        if f_code.is_empty() {
            continue;
        }
        next_code = Some(f_code);
        break;
    }

    let Some(nc) = next_code else {
        return default_indent;
    };

    let f_first_word = nc.split_whitespace().next().unwrap_or("");
    let is_elif = f_first_word == "elif" || nc.starts_with("elif(");
    let is_else = f_first_word == "else";
    let is_catch = f_first_word == "catch" || nc.starts_with("catch(");
    let is_finally = f_first_word == "finally" || nc.starts_with("finally(");
    let is_is = f_first_word == "is" || nc.starts_with("is(");

    // Find effective top block kind, ignoring Brace and Bracket
    let effective_top = block_stack
        .iter()
        .rev()
        .find(|&&b| b != BlockKind::Brace && b != BlockKind::Bracket);

    let is_continuation = match effective_top {
        Some(BlockKind::If) => is_elif || is_else,
        Some(BlockKind::Try) => is_catch || is_finally,
        Some(BlockKind::WhenArm) => is_is || is_else,
        _ => false,
    };

    if !is_continuation {
        return default_indent;
    }

    let continuation_indent = default_indent.saturating_sub(1);

    // Measure original indentation of this comment line (tabs counted as 4 spaces)
    let original_indent_spaces: usize = raw_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum();

    let group_preceded_by_blank =
        preceded_by_blank || is_comment_group_preceded_by_blank(lines, current_idx);

    // If author aligned with outer continuation block (or unindented), or if preceded by a blank line:
    if original_indent_spaces <= continuation_indent * 4 || group_preceded_by_blank {
        continuation_indent
    } else {
        default_indent
    }
}

/// Formats the given Alya source code string.
pub fn format_source(source: &str) -> Result<String, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut formatted_lines: Vec<String> = Vec::new();
    let mut block_stack: Vec<BlockKind> = Vec::new();
    let mut multiline_state: Option<MultilineLiteralState> = None;
    let mut prev_was_empty = false;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // 1. Multiline literal handling (string or comment): preserve lines verbatim
        if let Some(state) = multiline_state {
            formatted_lines.push(line.to_string());
            multiline_state = scan_line_multiline_state(line, Some(state));
            prev_was_empty = false;
            continue;
        }

        // Empty line handling
        if trimmed.is_empty() {
            if !prev_was_empty && !formatted_lines.is_empty() {
                formatted_lines.push(String::new());
                prev_was_empty = true;
            }
            continue;
        }
        let preceded_by_blank = prev_was_empty;
        prev_was_empty = false;

        let code = strip_line_comment(trimmed);

        // Pure comment line: preserve comment indentation with current block level or continuation branch
        if code.is_empty() {
            let indent_level =
                determine_comment_indent(&lines, line_idx, line, preceded_by_blank, &block_stack);
            let indent = " ".repeat(indent_level * 4);
            formatted_lines.push(format!("{}{}", indent, trimmed));
            multiline_state = scan_line_multiline_state(trimmed, None);
            continue;
        }

        let popped_bytes = pop_leading_closing_delimiters(code, &mut block_stack);

        let first_word = code.split_whitespace().next().unwrap_or("");
        let is_end = first_word == "end" || code.starts_with("end(");
        let is_elif = first_word == "elif" || code.starts_with("elif(");
        let is_else = first_word == "else";
        let is_is = (first_word == "is" || code.starts_with("is("))
            && block_stack
                .iter()
                .any(|b| matches!(b, BlockKind::When | BlockKind::WhenArm));
        let is_catch = first_word == "catch" || code.starts_with("catch(");
        let is_finally = first_word == "finally" || code.starts_with("finally(");

        let line_indent: usize;

        if is_end {
            if block_stack.last() == Some(&BlockKind::WhenArm) {
                block_stack.pop();
            }
            while let Some(top) = block_stack.last() {
                if matches!(top, BlockKind::Brace | BlockKind::Bracket) {
                    block_stack.pop();
                } else {
                    break;
                }
            }
            if !block_stack.is_empty() {
                block_stack.pop();
            }
            line_indent = block_stack.len();
        } else if is_elif {
            while let Some(top) = block_stack.last() {
                if matches!(top, BlockKind::Brace | BlockKind::Bracket) {
                    block_stack.pop();
                } else {
                    break;
                }
            }
            line_indent = block_stack.len().saturating_sub(1);
        } else if is_else {
            if block_stack.last() == Some(&BlockKind::WhenArm) {
                block_stack.pop();
            }
            while let Some(top) = block_stack.last() {
                if matches!(top, BlockKind::Brace | BlockKind::Bracket) {
                    block_stack.pop();
                } else {
                    break;
                }
            }
            if block_stack.last() == Some(&BlockKind::When) {
                line_indent = block_stack.len();
                let is_inline = is_when_else_inline(code);
                if !is_inline {
                    block_stack.push(BlockKind::WhenArm);
                }
            } else {
                line_indent = block_stack.len().saturating_sub(1);
            }
        } else if is_is {
            if block_stack.last() == Some(&BlockKind::WhenArm) {
                block_stack.pop();
            }
            line_indent = block_stack.len();
            let is_inline = is_when_arm_inline(code);
            if !is_inline {
                block_stack.push(BlockKind::WhenArm);
            }
        } else if is_catch || is_finally {
            while let Some(top) = block_stack.last() {
                if matches!(top, BlockKind::Brace | BlockKind::Bracket) {
                    block_stack.pop();
                } else {
                    break;
                }
            }
            line_indent = block_stack.len().saturating_sub(1);
        } else {
            line_indent = block_stack.len();
        }

        // Format code on the line
        let clean_content = format_line_content(trimmed);
        let indent_spaces = " ".repeat(line_indent * 4);
        formatted_lines.push(format!("{}{}", indent_spaces, clean_content));

        // Indent increase triggers (opens a new block for following lines)
        if !is_end && !is_is && !is_else && !is_elif && !is_catch && !is_finally {
            let in_extern = is_in_extern_block(&block_stack);
            if let Some(new_block) = get_block_starter(code, in_extern) {
                block_stack.push(new_block);
            }
        }

        if popped_bytes < code.len() {
            scan_delimiters_after_leading(&code[popped_bytes..], &mut block_stack);
        }

        multiline_state = scan_line_multiline_state(trimmed, None);
    }

    // Trim trailing empty lines so that the file ends cleanly with no trailing blank lines
    while formatted_lines.last().is_some_and(|s| s.is_empty()) {
        formatted_lines.pop();
    }

    let eol = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = formatted_lines.join(eol);
    if !result.is_empty() {
        result.push_str(eol);
    }

    // Safety Verification: Ensure formatted code parses successfully into valid AST
    let mut lexer = Lexer::new(&result);
    if let Ok(tokens) = lexer.tokenize() {
        let mut parser = Parser::new(tokens);
        if let Err(err) = parser.parse() {
            return Err(format!(
                "Formatter safety check failed: formatted code would produce parse error: {}",
                err
            ));
        }
    } else {
        return Err(
            "Formatter safety check failed: formatted code cannot be tokenized".to_string(),
        );
    }

    Ok(result)
}

fn format_line_content(line: &str) -> String {
    // If the line contains raw string delimiters, triple quotes, or multiline comments,
    // do not touch it to avoid any corruption of literal contents.
    if line.contains("\"\"\"") || line.contains('`') || line.contains("/*") || line.contains("*/") {
        return line.to_string();
    }

    // Format spacing while strictly preserving string literals and comments
    let mut out = String::new();
    let mut in_str = false;
    let mut quote_char = '"';
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_str {
            out.push(ch);
            if ch == '\\' {
                if let Some(next_ch) = chars.next() {
                    out.push(next_ch);
                }
            } else if ch == quote_char {
                in_str = false;
            }
            continue;
        }

        if ch == '"' || ch == '`' {
            in_str = true;
            quote_char = ch;
            out.push(ch);
            continue;
        }

        // Comments: leave everything until end of line intact
        if ch == '#' || (ch == '/' && chars.peek() == Some(&'/')) {
            out.push(ch);
            for rest in chars {
                out.push(rest);
            }
            break;
        }

        // Clean spacing around commas outside quotes
        if ch == ',' {
            out.push(',');
            if chars.peek() != Some(&' ') && chars.peek().is_some() {
                out.push(' ');
            }
            continue;
        }

        out.push(ch);
    }

    out
}

/// Recursively discovers all .alya files in a given path.
pub fn find_alya_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("alya") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.starts_with('.') || file_name == "target" || file_name == "build" {
                    continue;
                }
                if p.is_dir() {
                    files.extend(find_alya_files(&p));
                } else if p.extension().and_then(|ext| ext.to_str()) == Some("alya") {
                    files.push(p);
                }
            }
        }
    }
    files.sort();
    files
}

/// Formats a single file or directory. Returns Ok(number_of_changed_files).
pub fn run_fmt(path_str: &str, check_only: bool) -> Result<usize, String> {
    let root = Path::new(path_str);
    let files = find_alya_files(root);

    if files.is_empty() {
        println!("No .alya files found in '{}'.", path_str);
        return Ok(0);
    }

    let mut changed_count = 0;
    for file in &files {
        let display_path = file.display().to_string();
        let content = fs::read_to_string(file)
            .map_err(|e| format!("Error reading '{}': {}", display_path, e))?;

        let formatted = format_source(&content)
            .map_err(|e| format!("Error formatting '{}': {}", display_path, e))?;

        if formatted != content {
            changed_count += 1;
            if check_only {
                println!("  \x1b[1;33mneeds formatting\x1b[0m: {}", display_path);
            } else {
                fs::write(file, &formatted)
                    .map_err(|e| format!("Error writing formatted '{}': {}", display_path, e))?;
                println!("  \x1b[1;32mformatted\x1b[0m: {}", display_path);
            }
        } else if !check_only {
            println!("  \x1b[90malready formatted\x1b[0m: {}", display_path);
        }
    }

    if check_only {
        if changed_count > 0 {
            Err(format!(
                "Formatting check failed: {} file(s) need formatting.",
                changed_count
            ))
        } else {
            println!(
                "\x1b[1;32m✓ All {} file(s) are properly formatted.\x1b[0m",
                files.len()
            );
            Ok(0)
        }
    } else {
        println!(
            "\x1b[1;32m✓ Formatted {} of {} file(s).\x1b[0m",
            changed_count,
            files.len()
        );
        Ok(changed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_try_catch_bare() {
        let input = "try\nlet a = 1\ncatch\nsay \"err\"\nend\n";
        let expected = "try\n    let a = 1\ncatch\n    say \"err\"\nend\n";
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_try_catch_with_err_and_finally() {
        let input = "try\nsay \"try\"\ncatch err\nsay err\nfinally\nsay \"done\"\nend\n";
        let expected =
            "try\n    say \"try\"\ncatch err\n    say err\nfinally\n    say \"done\"\nend\n";
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_when_multiline() {
        let input = r#"when status
is 200, 201 then
say "ok"
is 400..499
say "client error"
else
say "unknown"
end
"#;
        let expected = r#"when status
    is 200, 201 then
        say "ok"
    is 400..499
        say "client error"
    else
        say "unknown"
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_when_inline() {
        let input = r#"when priority
is 1 then say "Low"
is 2 then say "Medium"
else say "Custom"
end
"#;
        let expected = r#"when priority
    is 1 then say "Low"
    is 2 then say "Medium"
    else say "Custom"
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_multiline_comment_preservation() {
        let input = "/*\n   Showcasing:\n   - Item 1\n   - Item 2\n*/\nlet a = 1\n";
        let expected = "/*\n   Showcasing:\n   - Item 1\n   - Item 2\n*/\nlet a = 1\n";
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_nested_function_with_when() {
        let input = r#"function check(val)
when val
is 1 then
say "one"
else
say "other"
end
end
"#;
        let expected = r#"function check(val)
    when val
        is 1 then
            say "one"
        else
            say "other"
    end
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_trailing_blank_lines_trimmed() {
        let input = "let x = 1\n\n\n\n";
        let expected = "let x = 1\n";
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_single_line_functions() {
        let input =
            "function foo() return 1 end\nfunction bar() return 2 end\nlet x = foo() + bar()\n";
        let expected =
            "function foo() return 1 end\nfunction bar() return 2 end\nlet x = foo() + bar()\n";
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_multiline_map() {
        let input = r#"function get_user()
let u = {
"name": "Alya",
"age": 2
}
return u
end
"#;
        let expected = r#"function get_user()
    let u = {
        "name": "Alya",
        "age": 2
    }
    return u
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_multiline_return_map() {
        let input = r#"function get_date()
return {
"year": 2026,
"month": 9
}
end
"#;
        let expected = r#"function get_date()
    return {
        "year": 2026,
        "month": 9
    }
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_multiline_array() {
        let input = r#"function get_items()
let items = [
"alpha",
"beta"
]
return items
end
"#;
        let expected = r#"function get_items()
    let items = [
        "alpha",
        "beta"
    ]
    return items
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_multiline_double_quote_preservation() {
        let input = r#"function main()
let sample = "
[package]
name = "test"
[[routes]]
path = "/api"
"
say "done"
end
"#;
        let expected = r#"function main()
    let sample = "
[package]
name = "test"
[[routes]]
path = "/api"
"
    say "done"
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_extern_c_block() {
        let input = r#"extern "C"
function puts(s: str) -> i32
function abs(n: i32) -> i32
end
"#;
        let expected = r#"extern "C"
    function puts(s: str) -> i32
    function abs(n: i32) -> i32
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_extern_c_from_lib() {
        let input = r#"extern "C" from "sqlite3"
function sqlite3_libversion() -> str
function sqlite3_sourceid() -> str
function sqlite3_open(filename: str, pp_db: ptr) -> i32
function sqlite3_close(db: ptr) -> i32
end
"#;
        let expected = r#"extern "C" from "sqlite3"
    function sqlite3_libversion() -> str
    function sqlite3_sourceid() -> str
    function sqlite3_open(filename: str, pp_db: ptr) -> i32
    function sqlite3_close(db: ptr) -> i32
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_comment_before_elif_preserved() {
        let input = r#"function check(cp)
    # Wide CJK
    if cp >= 4352
        count += 2
    # Common 3-byte Emojis
    elif cp >= 9200
        count += 2
    else
        count += 1
    end
end
"#;
        let expected = r#"function check(cp)
    # Wide CJK
    if cp >= 4352
        count += 2
    # Common 3-byte Emojis
    elif cp >= 9200
        count += 2
    else
        count += 1
    end
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_comment_inside_if_preserved() {
        let input = r#"function check(cp)
    if cp >= 4352
        count += 2
        # note: strictly inside if
    elif cp >= 9200
        count += 2
    end
end
"#;
        let expected = r#"function check(cp)
    if cp >= 4352
        count += 2
        # note: strictly inside if
    elif cp >= 9200
        count += 2
    end
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_comment_before_else_and_catch() {
        let input = r#"function run()
    try
        connect()
    # Handle connection error
    catch e
        log(e)
    # Cleanup always
    finally
        cleanup()
    end
end
"#;
        let expected = r#"function run()
    try
        connect()
    # Handle connection error
    catch e
        log(e)
    # Cleanup always
    finally
        cleanup()
    end
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_enum() {
        let input = r#"enum Color
Red
Green
Blue = 10
end
"#;
        let expected = r#"enum Color
    Red
    Green
    Blue = 10
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_pub_declarations() {
        let input = r#"pub function add(a: int, b: int) -> int
return a + b
end

pub struct Point
x: int
y: int = 0
end

pub enum Direction
North
South
end
"#;
        let expected = r#"pub function add(a: int, b: int) -> int
    return a + b
end

pub struct Point
    x: int
    y: int = 0
end

pub enum Direction
    North
    South
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_when_expression_with_arrow() {
        let input = r#"let res = when val
is 1 => "one"
is 2 => "two"
else => "other"
end
"#;
        let expected = r#"let res = when val
    is 1 => "one"
    is 2 => "two"
    else => "other"
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }

    #[test]
    fn test_format_new_syntax_features() {
        let input = r#"# 1. in / not in
let has_x = 5 in [1, 2, 5]
let no_y = "z" not in "hello"

# 2. multiple loop variables
for k, v in my_map
say k
say v
end

# 3. destructuring & spread
let [a, b, ...rest] = items
let { x, y } = point
let merged = [...a, ...b]

# 4. type check
if val is int
say "integer"
elif val is not string
say "not string"
end
"#;
        let expected = r#"# 1. in / not in
let has_x = 5 in [1, 2, 5]
let no_y = "z" not in "hello"

# 2. multiple loop variables
for k, v in my_map
    say k
    say v
end

# 3. destructuring & spread
let [a, b, ...rest] = items
let { x, y } = point
let merged = [...a, ...b]

# 4. type check
if val is int
    say "integer"
elif val is not string
    say "not string"
end
"#;
        assert_eq!(format_source(input).unwrap(), expected);
    }
}
