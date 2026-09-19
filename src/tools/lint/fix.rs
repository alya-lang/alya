use super::types::{LintDiagnostic, LintFix};

/// Converts 1-indexed (line, col) to a byte offset in the given source string.
pub fn line_col_to_byte_offset(source: &str, line: usize, col: usize) -> Option<usize> {
    if line == 0 || col == 0 {
        return None;
    }
    let mut current_line = 1;
    let mut offset = 0;

    for (i, b) in source.bytes().enumerate() {
        if current_line == line {
            // Found line, advance by (col - 1) bytes or characters
            // Let's count characters in this line
            let line_slice = &source[offset..];
            let mut char_count = 1;
            for (byte_idx, _) in line_slice.char_indices() {
                if char_count == col {
                    return Some(offset + byte_idx);
                }
                char_count += 1;
            }
            return Some(offset + line_slice.len());
        }
        if b == b'\n' {
            current_line += 1;
            offset = i + 1;
        }
    }

    if current_line == line {
        let line_slice = &source[offset..];
        let mut char_count = 1;
        for (byte_idx, _) in line_slice.char_indices() {
            if char_count == col {
                return Some(offset + byte_idx);
            }
            char_count += 1;
        }
        return Some(offset + line_slice.len());
    }

    None
}

/// Applies all available fixes in diagnostics to the source string.
/// Fixes are sorted from bottom to top (highest byte offset first) to prevent offsetting.
/// Returns (new_source, number_of_fixes_applied).
pub fn apply_fixes_to_source(source: &str, diagnostics: &[LintDiagnostic]) -> (String, usize) {
    let mut fixes: Vec<&LintFix> = diagnostics.iter().filter_map(|d| d.fix.as_ref()).collect();
    if fixes.is_empty() {
        return (source.to_string(), 0);
    }

    // Map fixes to (start_byte, end_byte, replacement)
    let mut byte_fixes = Vec::new();
    for fix in fixes.drain(..) {
        if let (Some(start), Some(end)) = (
            line_col_to_byte_offset(source, fix.start_line, fix.start_col),
            line_col_to_byte_offset(source, fix.end_line, fix.end_col),
        ) {
            if start <= end && end <= source.len() {
                byte_fixes.push((start, end, &fix.replacement));
            }
        }
    }

    // Sort by start descending so edits don't invalidate earlier offsets
    byte_fixes.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result = source.to_string();
    let mut applied_count = 0;
    let mut last_start = usize::MAX;

    for (start, end, replacement) in byte_fixes {
        // Prevent overlapping fixes
        if end <= last_start {
            result.replace_range(start..end, replacement);
            last_start = start;
            applied_count += 1;
        }
    }

    (result, applied_count)
}
