use super::types::LintDiagnostic;
use std::collections::{HashMap, HashSet};

/// Stores inline suppression instructions found in comment directives.
#[derive(Debug, Default)]
pub struct SuppressionFilter {
    /// line_number -> Set of rule names disabled on this line (None means all rules).
    disabled_lines: HashMap<usize, Option<HashSet<String>>>,
    /// Range of lines where rules are disabled: (start_line, end_line, Option<HashSet<rule_names>>)
    disabled_ranges: Vec<(usize, usize, Option<HashSet<String>>)>,
}

impl SuppressionFilter {
    /// Parses suppression comments from the source string.
    pub fn from_source(source: &str) -> Self {
        let mut filter = Self::default();
        let mut active_ranges: Vec<(usize, Option<HashSet<String>>)> = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_no = line_idx + 1;

            if let Some(pos) = line.find('#') {
                let comment = line[pos + 1..].trim();

                // 1. disable-next-line
                if let Some(rest) = comment
                    .strip_prefix("alya-lint: disable-next-line")
                    .or_else(|| comment.strip_prefix("alya-lint:disable-next-line"))
                    .or_else(|| comment.strip_prefix("alya-ignore-next-line"))
                    .or_else(|| comment.strip_prefix("alya-ignore: next-line"))
                {
                    let rules = parse_rule_list(rest);
                    filter.disabled_lines.insert(line_no + 1, rules);
                    continue;
                }

                // 2. disable / enable range
                if let Some(rest) = comment
                    .strip_prefix("alya-lint: disable")
                    .or_else(|| comment.strip_prefix("alya-lint:disable"))
                {
                    let rules = parse_rule_list(rest);
                    active_ranges.push((line_no, rules));
                    continue;
                }

                if let Some(rest) = comment
                    .strip_prefix("alya-lint: enable")
                    .or_else(|| comment.strip_prefix("alya-lint:enable"))
                {
                    let rules = parse_rule_list(rest);
                    if let Some(idx) = active_ranges.iter().rposition(|(_, r)| *r == rules) {
                        let (start, r) = active_ranges.remove(idx);
                        filter.disabled_ranges.push((start, line_no, r));
                    }
                    continue;
                }

                // 3. same-line ignore: e.g. `let x = 1 # alya-ignore` or `# alya-lint: ignore`
                if let Some(rest) = comment
                    .strip_prefix("alya-ignore:")
                    .or_else(|| comment.strip_prefix("alya-ignore"))
                    .or_else(|| comment.strip_prefix("alya-lint: ignore"))
                    .or_else(|| comment.strip_prefix("alya-lint:ignore"))
                {
                    let rules = parse_rule_list(rest);
                    filter.disabled_lines.insert(line_no, rules);
                }
            }
        }

        // Close any unclosed disable ranges to the end of file (usize::MAX)
        for (start, rules) in active_ranges {
            filter.disabled_ranges.push((start, usize::MAX, rules));
        }

        filter
    }

    /// Checks if a diagnostic at the given line with the given rule is suppressed.
    pub fn is_suppressed(&self, line: usize, rule: &str) -> bool {
        // Check exact line suppression
        if let Some(rules_opt) = self.disabled_lines.get(&line) {
            match rules_opt {
                None => return true, // all rules suppressed
                Some(rules) => {
                    if rules.contains(rule) {
                        return true;
                    }
                }
            }
        }

        // Check range suppression
        for (start, end, rules_opt) in &self.disabled_ranges {
            if line >= *start && line <= *end {
                match rules_opt {
                    None => return true,
                    Some(rules) => {
                        if rules.contains(rule) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Filters out all suppressed diagnostics.
    pub fn filter_diagnostics(&self, diags: Vec<LintDiagnostic>) -> Vec<LintDiagnostic> {
        diags
            .into_iter()
            .filter(|d| !self.is_suppressed(d.line, &d.rule))
            .collect()
    }
}

fn parse_rule_list(s: &str) -> Option<HashSet<String>> {
    let s = s.trim().trim_start_matches(':').trim();
    if s.is_empty() {
        return None;
    }
    let mut set = HashSet::new();
    for part in s.split([',', ' ']) {
        let p = part.trim();
        if !p.is_empty() {
            set.insert(p.to_string());
        }
    }
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}
