use super::Lexer;

impl Lexer {
    pub(crate) fn read_number(&mut self) -> Result<(f64, bool), String> {
        let start_line = self.line;
        let start_col = self.column;

        // Check for 0x (hex), 0b (binary), 0o (octal)
        if self.current_char() == Some('0') {
            if let Some(p) = self.peek_char() {
                if p == 'x' || p == 'X' {
                    self.advance(); // skip '0'
                    self.advance(); // skip 'x'
                    let mut s = String::new();
                    while let Some(ch) = self.current_char() {
                        if ch.is_ascii_hexdigit() {
                            s.push(ch);
                            self.advance();
                        } else if ch == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() {
                        return Err(format!(
                            "Expected hex digits after '0x' at line {}, column {}",
                            start_line, start_col
                        ));
                    }
                    let val = u64::from_str_radix(&s, 16).map_err(|_| {
                        format!(
                            "Invalid hexadecimal number '0x{}' at line {}, column {}",
                            s, start_line, start_col
                        )
                    })?;
                    return Ok((val as f64, false));
                } else if p == 'b' || p == 'B' {
                    self.advance(); // skip '0'
                    self.advance(); // skip 'b'
                    let mut s = String::new();
                    while let Some(ch) = self.current_char() {
                        if ch == '0' || ch == '1' {
                            s.push(ch);
                            self.advance();
                        } else if ch == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() {
                        return Err(format!(
                            "Expected binary digits after '0b' at line {}, column {}",
                            start_line, start_col
                        ));
                    }
                    let val = u64::from_str_radix(&s, 2).map_err(|_| {
                        format!(
                            "Invalid binary number '0b{}' at line {}, column {}",
                            s, start_line, start_col
                        )
                    })?;
                    return Ok((val as f64, false));
                } else if p == 'o' || p == 'O' {
                    self.advance(); // skip '0'
                    self.advance(); // skip 'o'
                    let mut s = String::new();
                    while let Some(ch) = self.current_char() {
                        if matches!(ch, '0'..='7') {
                            s.push(ch);
                            self.advance();
                        } else if ch == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() {
                        return Err(format!(
                            "Expected octal digits after '0o' at line {}, column {}",
                            start_line, start_col
                        ));
                    }
                    let val = u64::from_str_radix(&s, 8).map_err(|_| {
                        format!(
                            "Invalid octal number '0o{}' at line {}, column {}",
                            s, start_line, start_col
                        )
                    })?;
                    return Ok((val as f64, false));
                }
            }
        }

        let mut has_dot = false;
        let mut s = String::new();

        if self.current_char() == Some('.') {
            has_dot = true;
            s.push('.');
            self.advance();
        }

        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance();
            } else if ch == '.' && !has_dot && self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                has_dot = true;
                s.push('.');
                self.advance();
            } else if (ch == 'e' || ch == 'E') && !s.is_empty() {
                has_dot = true;
                s.push(ch);
                self.advance();
                if let Some(sign) = self.current_char() {
                    if sign == '+' || sign == '-' {
                        s.push(sign);
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        let val = s.parse::<f64>().map_err(|_| {
            format!(
                "Invalid number '{}' at line {}, column {}",
                s, start_line, start_col
            )
        })?;
        Ok((val, has_dot))
    }

    pub(crate) fn read_string(&mut self) -> Result<String, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening quote
        let mut result = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // Skip closing quote
                return Ok(result);
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('e') => result.push('\x1b'),
                    Some('a') => result.push('\x07'),
                    Some('b') => result.push('\x08'),
                    Some('f') => result.push('\x0c'),
                    Some('v') => result.push('\x0b'),
                    Some('0') => result.push('\0'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('{') => result.push('{'),
                    Some('}') => result.push('}'),
                    Some(c) => result.push(c),
                    None => {
                        return Err(format!(
                            "Unexpected end of string at line {}, column {}",
                            self.line, self.column
                        ))
                    }
                }
                self.advance();
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(format!(
            "Unterminated string starting at line {}, column {}",
            start_line, start_col
        ))
    }

    pub(crate) fn read_format_string(&mut self) -> Result<String, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening quote
        let mut result = String::new();
        let mut brace_depth = 0;

        while let Some(ch) = self.current_char() {
            if brace_depth == 0 && ch == '"' {
                self.advance(); // Skip closing quote
                return Ok(result);
            } else if brace_depth > 0 && ch == '"' {
                // Nested string inside { ... }
                result.push('"');
                self.advance();
                while let Some(inner_ch) = self.current_char() {
                    if inner_ch == '"' {
                        result.push('"');
                        self.advance();
                        break;
                    } else if inner_ch == '\\' {
                        result.push('\\');
                        self.advance();
                        if let Some(esc) = self.current_char() {
                            result.push(esc);
                            self.advance();
                        }
                    } else {
                        result.push(inner_ch);
                        self.advance();
                    }
                }
            } else if ch == '{' {
                if self.peek_char() == Some('{') && brace_depth == 0 {
                    result.push('{');
                    result.push('{');
                    self.advance();
                    self.advance();
                } else {
                    brace_depth += 1;
                    result.push('{');
                    self.advance();
                }
            } else if ch == '}' {
                if self.peek_char() == Some('}') && brace_depth == 0 {
                    result.push('}');
                    result.push('}');
                    self.advance();
                    self.advance();
                } else {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                    result.push('}');
                    self.advance();
                }
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('e') => result.push('\x1b'),
                    Some('a') => result.push('\x07'),
                    Some('b') => result.push('\x08'),
                    Some('f') => result.push('\x0c'),
                    Some('v') => result.push('\x0b'),
                    Some('0') => result.push('\0'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('{') => result.push('{'),
                    Some('}') => result.push('}'),
                    Some(c) => result.push(c),
                    None => {
                        return Err(format!(
                            "Unexpected end of string at line {}, column {}",
                            self.line, self.column
                        ))
                    }
                }
                self.advance();
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(format!(
            "Unterminated string starting at line {}, column {}",
            start_line, start_col
        ))
    }

    pub(crate) fn read_triple_quoted_string(&mut self) -> Result<String, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance();
        self.advance();
        self.advance();

        if self.current_char() == Some('\r') && self.peek_char() == Some('\n') {
            self.advance();
            self.advance();
        } else if self.current_char() == Some('\n') {
            self.advance();
        }

        let mut result = String::new();
        while let Some(ch) = self.current_char() {
            if ch == '"' && self.peek_char() == Some('"') && self.peek_char_at(2) == Some('"') {
                self.advance();
                self.advance();
                self.advance();
                return Ok(result);
            } else if ch == '\r' {
                self.advance();
                if self.current_char() == Some('\n') {
                    self.advance();
                }
                result.push('\n');
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('e') => result.push('\x1b'),
                    Some('a') => result.push('\x07'),
                    Some('b') => result.push('\x08'),
                    Some('f') => result.push('\x0c'),
                    Some('v') => result.push('\x0b'),
                    Some('0') => result.push('\0'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('{') => result.push('{'),
                    Some('}') => result.push('}'),
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => {
                        return Err(format!(
                            "Unexpected end of string at line {}, column {}",
                            self.line, self.column
                        ));
                    }
                }
                self.advance();
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(format!(
            "Unterminated multiline string starting at line {}, column {}",
            start_line, start_col
        ))
    }

    pub(crate) fn read_raw_string(&mut self) -> Result<String, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening `
        let mut result = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '`' {
                self.advance(); // Skip closing `
                return Ok(result);
            } else if ch == '\r' {
                self.advance();
                if self.current_char() == Some('\n') {
                    self.advance();
                }
                result.push('\n');
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(format!(
            "Unterminated raw string starting at line {}, column {}",
            start_line, start_col
        ))
    }

    pub(crate) fn read_identifier(&mut self) -> String {
        let start_pos = self.position;

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        self.input[start_pos..self.position].iter().collect()
    }

    pub(crate) fn read_rune(&mut self) -> Result<char, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening '

        let ch = match self.current_char() {
            Some('\\') => {
                self.advance();
                match self.current_char() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('0') => '\0',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some('"') => '"',
                    Some('e') => '\x1b',
                    Some('u') => {
                        self.advance();
                        if self.current_char() == Some('{') {
                            self.advance();
                            let hex_start = self.position;
                            while let Some(c) = self.current_char() {
                                if c == '}' {
                                    break;
                                }
                                self.advance();
                            }
                            let hex_str: String = self.input[hex_start..self.position].iter().collect();
                            self.advance(); // skip '}'
                            let codepoint = u32::from_str_radix(&hex_str, 16).map_err(|_| {
                                format!(
                                    "Invalid unicode escape '\\u{{{}}}' at line {}, column {}",
                                    hex_str, start_line, start_col
                                )
                            })?;
                            char::from_u32(codepoint).ok_or_else(|| {
                                format!(
                                    "Invalid unicode codepoint {:X} at line {}, column {}",
                                    codepoint, start_line, start_col
                                )
                            })?
                        } else {
                            return Err(format!(
                                "Expected '{{' after '\\u' at line {}, column {}",
                                start_line, start_col
                            ));
                        }
                    }
                    Some(c) => c,
                    None => {
                        return Err(format!(
                            "Unexpected end of rune literal at line {}, column {}",
                            self.line, self.column
                        ));
                    }
                }
            }
            Some('\'') => {
                return Err(format!(
                    "Empty rune literal at line {}, column {}",
                    start_line, start_col
                ));
            }
            Some(c) => c,
            None => {
                return Err(format!(
                    "Unexpected end of rune literal at line {}, column {}",
                    self.line, self.column
                ));
            }
        };

        self.advance(); // Skip char

        if self.current_char() != Some('\'') {
            return Err(format!(
                "Unterminated rune literal starting at line {}, column {}",
                start_line, start_col
            ));
        }
        self.advance(); // Skip closing '

        Ok(ch)
    }

    pub(crate) fn read_raw_quoted_string(&mut self) -> Result<String, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening "
        let mut result = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // Skip closing "
                return Ok(result);
            } else if ch == '\r' {
                self.advance();
                if self.current_char() == Some('\n') {
                    self.advance();
                }
                result.push('\n');
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(format!(
            "Unterminated raw string starting at line {}, column {}",
            start_line, start_col
        ))
    }
}
