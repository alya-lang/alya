use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Warning,
    Info,
    Error,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintSeverity::Warning => write!(f, "warning"),
            LintSeverity::Info => write!(f, "info"),
            LintSeverity::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LintFormat {
    #[default]
    Text,
    Sarif,
}

impl LintFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Some(LintFormat::Text),
            "sarif" => Some(LintFormat::Sarif),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFix {
    pub description: String,
    pub replacement: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub rule: String,
    pub severity: LintSeverity,
    pub message: String,
    pub file_path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub help: Option<String>,
    pub fix: Option<LintFix>,
}

impl LintDiagnostic {
    pub fn render(&self, source: &str) -> String {
        let mut out = String::new();
        let color_code = match self.severity {
            LintSeverity::Warning => "\x1b[1;33m",
            LintSeverity::Info => "\x1b[1;36m",
            LintSeverity::Error => "\x1b[1;31m",
        };
        let reset = "\x1b[0m";

        out.push_str(&format!(
            "{}{}[{}]{}: {}\n",
            color_code, self.severity, self.rule, reset, self.message
        ));

        let display_path = self.file_path.display().to_string().replace('\\', "/");
        out.push_str(&format!(
            "  \x1b[1;34m-->\x1b[0m {}:{}:{}\n",
            display_path, self.line, self.col
        ));

        let lines: Vec<&str> = source.lines().collect();
        if self.line > 0 && self.line <= lines.len() {
            let line_str = lines[self.line - 1];
            let width = self.line.to_string().len().max(1);

            out.push_str(&format!("  {:>width$} |\n", "", width = width));

            let display_line = line_str.replace('\t', "    ");
            out.push_str(&format!(
                "  {:>width$} | {}\n",
                self.line,
                display_line,
                width = width
            ));

            let mut visual_col = 0;
            for (idx, ch) in line_str.chars().enumerate() {
                if idx + 1 == self.col {
                    break;
                }
                if ch == '\t' {
                    visual_col += 4;
                } else {
                    visual_col += 1;
                }
            }

            let span_len = if self.end_line == self.line && self.end_col >= self.col {
                (self.end_col - self.col).max(1)
            } else {
                1
            };

            let carets = "^".repeat(span_len);
            out.push_str(&format!(
                "  {:>width$} | {}{}{}{}\n",
                "",
                " ".repeat(visual_col),
                color_code,
                carets,
                reset,
                width = width
            ));

            if let Some(help) = &self.help {
                out.push_str(&format!(
                    "  {:>width$} | \x1b[1;36m= help:\x1b[0m {}\n",
                    "",
                    help,
                    width = width
                ));
            }
        }

        out
    }
}

#[derive(Debug, Default, Clone)]
pub struct LintReport {
    pub files_scanned: usize,
    pub files_with_issues: usize,
    pub total_diagnostics: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub info_count: usize,
    pub fixes_applied: usize,
    pub diagnostics: Vec<LintDiagnostic>,
}
