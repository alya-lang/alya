//! Peephole optimizer for generated assembly text.
//!
//! Operates on the emitted assembly lines (backend-agnostic AT&T syntax),
//! gated by profile `opt-level`: 0 returns the input untouched (byte
//! identity with unoptimized output is a hard guarantee — the default
//! `dev` profile must never change shape), 1+ runs the pass pipeline.
//!
//! Tiers (cumulative):
//! - O1: push/pop cancellation, push/pop→mov, self-move elimination,
//!   dead code after unconditional `ret`/`jmp`, jump-to-next-label.
//! - O2: O1 + straight-line redundant-load elimination.
//! - O3: O2 (reserved extension point: LICM, unrolling, vectorization).
//!
//! Safety rules (read before extending):
//! - Push/pop rewrites apply only to adjacent pairs naming registers;
//!   neither instruction touches flags or memory beyond the stack slot,
//!   and adjacency means no intervening stack operation exists.
//! - Dead-code skipping never crosses labels and never skips directives
//!   (lines starting with `.`): unwind data and section switches survive.
//! - Redundant-load elimination scans straight-line windows only: any
//!   `call`, memory store, destination-register write, label, or branch
//!   aborts the window. Loads never write flags, so flag state is safe.
//! - `mov` self-moves (`mov %rax, %rax`) are unconditional no-ops.

/// Pass tiers selected by profile `opt-level`.
pub fn passes_for(opt_level: u8) -> &'static [&'static str] {
    match opt_level {
        0 => &[],
        1 => &["o1"],
        _ => &["o1", "o2"],
    }
}

/// Runs the enabled passes over emitted assembly, returning the
/// (possibly) rewritten text. Level 0 is the identity function.
pub fn optimize_asm(asm: &str, opt_level: u8) -> String {
    let passes = passes_for(opt_level);
    if passes.is_empty() {
        return asm.to_string();
    }
    let mut lines: Vec<String> = asm.lines().map(|l| l.to_string()).collect();
    // O1 group first (it creates new adjacencies), then O2 to a fixpoint
    // (each shortening iteration enables the next; capped for safety).
    lines = pass_o1(lines);
    if passes.contains(&"o2") {
        for _ in 0..4 {
            let len_before = lines.len();
            lines = pass_o2_redundant_loads(lines);
            if lines.len() == len_before {
                break;
            }
        }
    }
    let mut out = lines.join("\n");
    if !out.is_empty() && asm.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Splits a line into (label, mnemonic, operands): at most one constituent
/// is Some. Directives (`.text`) report as mnemonic starting with `.`.
fn split_line(line: &str) -> (Option<&str>, Option<&str>, Option<&str>) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return (None, None, None);
    }
    // Any `name:` (including GAS local labels like `.L1:`) is a label.
    // Directives never end with a colon.
    if trimmed.ends_with(':') {
        return (Some(trimmed), None, None);
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap_or("");
    if mnemonic.is_empty() {
        return (None, None, None);
    }
    let operands = parts.next().map(str::trim).filter(|s| !s.is_empty());
    (None, Some(mnemonic), operands)
}

fn is_directive_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('.')
}

/// Normalizes AT&T size suffixes, but ONLY for the families this pass
/// reasons about. A blind trailing-`q`/`l` strip would corrupt `call`
/// into `cal` (silently disabling call-aware guards), so anything else
/// keeps its exact spelling (conservative: fewer matches, never wrong).
fn base_mnemonic(mnemonic: &str) -> &str {
    match mnemonic {
        "push" | "pushq" | "pushl" => "push",
        "pop" | "popq" | "popl" => "pop",
        "mov" | "movq" | "movl" | "movb" | "movw" => "mov",
        other => other,
    }
}

/// `Some(reg)` for `%reg` operands, else None.
fn as_register(operand: &str) -> Option<&str> {
    let op = operand.trim();
    if let Some(reg) = op.strip_prefix('%') {
        if !reg.is_empty() && reg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Some(reg);
        }
    }
    None
}

fn split_operands(operands: &str) -> Vec<&str> {
    operands.split(',').map(str::trim).collect()
}

/// O1 group: push/pop cancellation + folding, self-moves, dead code,
/// jump-to-next-label. Single left-to-right sweep.
fn pass_o1(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut unreachable = false;
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let (label, mnemonic, operands) = split_line(line);
        if label.is_some() {
            unreachable = false;
            out.push(line.clone());
            i += 1;
            continue;
        }
        if unreachable {
            // Directives always survive (unwind data, section switches).
            if is_directive_line(line) || split_line(line).0.is_some() {
                out.push(line.clone());
            }
            i += 1;
            continue;
        }
        let Some(mn) = mnemonic else {
            out.push(line.clone());
            i += 1;
            continue;
        };
        let base = base_mnemonic(mn);
        // Dead code starts after unconditional control transfer.
        if base == "ret" || base == "jmp" {
            // Jump-to-next-label: `jmp .Lx` directly followed by `.Lx:`.
            if base == "jmp" {
                if let Some(ops) = operands {
                    let targets = split_operands(ops);
                    if targets.len() == 1 {
                        let mut j = i + 1;
                        while j < lines.len() && lines[j].trim().is_empty() {
                            j += 1;
                        }
                        if j < lines.len() {
                            let (next_label, _, _) = split_line(&lines[j]);
                            let next_name = next_label.map(|l| l.trim_end_matches(':'));
                            if next_name == Some(targets[0].trim()) {
                                // Drop the jump; the label ends unreachable
                                // state on the next iteration anyway.
                                i += 1;
                                continue;
                            }
                        }
                    }
                }
            }
            out.push(line.clone());
            unreachable = true;
            i += 1;
            continue;
        }
        // push/pop pairs with the following line.
        if (base == "push" || base == "pop") && i + 1 < lines.len() {
            if let Some(next) = fold_push_pop(line, &lines[i + 1]) {
                if !next.is_empty() {
                    out.push(next);
                }
                i += 2;
                continue;
            }
        }
        // Self-move: `mov %rax, %rax`.
        if base == "mov" {
            if let Some(ops) = operands {
                let parts = split_operands(ops);
                if parts.len() == 2 && parts[0] == parts[1] {
                    i += 1;
                    continue;
                }
            }
        }
        out.push(line.clone());
        i += 1;
    }
    out
}

/// Folds adjacent `push X` / `pop Y`: same register cancels out,
/// otherwise becomes one `mov`. Returns None when the pair does not
/// match (caller keeps both lines).
fn fold_push_pop(push_line: &str, pop_line: &str) -> Option<String> {
    let (_, push_mn, push_ops) = split_line(push_line);
    let (_, pop_mn, pop_ops) = split_line(pop_line);
    if base_mnemonic(push_mn?) != "push" || base_mnemonic(pop_mn?) != "pop" {
        return None;
    }
    let src = as_register(push_ops?)?;
    let dst = as_register(pop_ops?)?;
    if src == dst {
        return Some(String::new());
    }
    let indent_len = push_line.len() - push_line.trim_start().len();
    let indent = &push_line[..indent_len];
    Some(format!("{}mov %{}, %{}", indent, src, dst))
}

/// Mnemonic families with implicit register writes beyond their
/// explicit destination operand (`mul`/`div` clobber `%rax`/`%rdx`).
/// Size-suffixed forms (`mulq`, `divl`) match; SSE lookalikes
/// (`divsd` writes only its explicit destination) do not.
fn has_implicit_writes(mn: &str) -> bool {
    [
        "mul", "imul", "div", "idiv", "xchg", "cmpxchg", "cpuid", "rdtsc",
    ]
    .iter()
    .any(|prefix| {
        mn == *prefix
            || (mn.starts_with(*prefix)
                && mn[prefix.len()..].chars().all(|c| "bwlq".contains(c))
                && mn.len() > prefix.len())
    })
}

/// Classifies a source operand: immediates never change, registers
/// change on writes, memory may alias any store.
fn src_kind(src: &str) -> char {
    if src.starts_with('$') {
        'i'
    } else if src.contains('(') {
        'm'
    } else if as_register(src).is_some() {
        'r'
    } else {
        '?'
    }
}

/// O2: straight-line redundant-load elimination. A `mov SRC, %R`
/// whose exact text reappears within a short window — with no intervening
/// call, memory store, destination write, SOURCE write, label, or
/// branch — drops the second occurrence.
///
/// The source-write check is load-bearing: `mov %rbx, %xmm1` followed by
/// `movabs $const, %rbx` and the same load again reads a DIFFERENT value.
/// Likewise, mnemonics with implicit register writes (`mul`/`div` family
/// clobber `%rax`/`%rdx`, `xchg` both sides) end the window outright.
fn pass_o2_redundant_loads(lines: Vec<String>) -> Vec<String> {
    const WINDOW: usize = 16;
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let (label, mnemonic, operands) = split_line(&lines[i]);
        let dominated = if label.is_none() {
            mnemonic
                .map(base_mnemonic)
                .filter(|m| *m == "mov")
                .and_then(|_| {
                    operands.and_then(|ops| {
                        let parts = split_operands(ops);
                        if parts.len() == 2 && as_register(parts[1]).is_some() {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    })
                })
        } else {
            None
        };
        let dominated = match dominated {
            Some(d) => d,
            None => {
                out.push(lines[i].clone());
                i += 1;
                continue;
            }
        };
        let (src, dst) = dominated;
        let skind = src_kind(&src);
        let mut redundant_at: Option<usize> = None;
        let mut j = i + 1;
        while j < lines.len() && j - i <= WINDOW {
            let (l2, m2, o2) = split_line(&lines[j]);
            if l2.is_some() {
                break;
            }
            let Some(mn) = m2.map(base_mnemonic) else {
                j += 1;
                continue;
            };
            // Control flow, calls, stack writes, and implicit register
            // writes end the window.
            if mn == "call"
                || mn.starts_with('j')
                || mn == "ret"
                || mn == "push"
                || mn == "pop"
                || has_implicit_writes(mn)
            {
                break;
            }
            // Exact duplicate load: redundant (checked before the
            // destination-write rule below, which would also match it).
            if mn == "mov" {
                if let Some(ops) = o2 {
                    let parts = split_operands(ops);
                    if parts.len() == 2 && parts[0] == src && parts[1] == dst {
                        redundant_at = Some(j);
                        break;
                    }
                }
            }
            // Any write to the destination register ends it (except
            // flag-only `cmp`/`test`, which write nothing). A write to a
            // SOURCE register, or any memory store when the source is
            // memory, ends it too. Immediates never change.
            if let Some(ops) = o2 {
                let parts = split_operands(ops);
                if mn != "cmp" && mn != "test" {
                    if let Some(last) = parts.last() {
                        if *last == dst {
                            break;
                        }
                        let last_is_mem = last.contains('(');
                        let last_is_reg = as_register(last).is_some();
                        if last_is_mem && skind == 'm' {
                            break;
                        }
                        if last_is_reg && skind == 'r' && *last == src {
                            break;
                        }
                    }
                }
            }
            j += 1;
        }
        out.push(lines[i].clone());
        i += 1;
        if let Some(r) = redundant_at {
            // Copy the (kept) lines between, skipping the duplicate.
            let mut k = i;
            while k <= r {
                if k != r {
                    out.push(lines[k].clone());
                }
                k += 1;
            }
            i = r + 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_zero_is_identity() {
        let asm = "fn_f:\n    push %rax\n    pop %rax\n    ret\n";
        assert_eq!(optimize_asm(asm, 0), asm);
    }

    #[test]
    fn push_pop_same_register_cancels() {
        let out = optimize_asm("    push %rax\n    pop %rax\n", 1);
        assert_eq!(out, "");
    }

    #[test]
    fn push_pop_folds_to_mov() {
        let out = optimize_asm("    push %rax\n    pop %rcx\n", 1);
        assert_eq!(out, "    mov %rax, %rcx\n");
    }

    #[test]
    fn push_pop_non_registers_untouched() {
        let asm = "    push $1\n    pop %rax\n";
        assert_eq!(optimize_asm(asm, 1), asm);
    }

    #[test]
    fn self_move_dropped() {
        let out = optimize_asm("    mov %rax, %rax\n", 1);
        assert_eq!(out, "");
    }

    #[test]
    fn dead_code_after_ret_skipped() {
        let asm = "    ret\n    mov %rax, %rbx\n.L1:\n    ret\n";
        let out = optimize_asm(asm, 1);
        assert_eq!(out, "    ret\n.L1:\n    ret\n");
    }

    #[test]
    fn directives_survive_dead_regions() {
        let asm = "    ret\n    .cfi_endproc\n.L1:\n    ret\n";
        let out = optimize_asm(asm, 1);
        assert!(out.contains(".cfi_endproc"));
    }

    #[test]
    fn jump_to_next_label_dropped() {
        let asm = "    jmp .L1\n.L1:\n    ret\n";
        let out = optimize_asm(asm, 1);
        assert_eq!(out, ".L1:\n    ret\n");
    }

    #[test]
    fn conditional_jumps_kept() {
        let asm = "    jg .L1\n    ret\n";
        assert_eq!(optimize_asm(asm, 1), asm);
    }

    #[test]
    fn adjacent_duplicate_load_dropped_at_o2() {
        let asm = "    mov -8(%rbp), %rax\n    mov -8(%rbp), %rax\n";
        let out = optimize_asm(asm, 2);
        assert_eq!(out, "    mov -8(%rbp), %rax\n");
        // O1 leaves it alone.
        assert_eq!(optimize_asm(asm, 1), asm);
    }

    #[test]
    fn call_aborts_load_window() {
        let asm = "    mov -8(%rbp), %rax\n    call fn_f\n    mov -8(%rbp), %rax\n";
        assert_eq!(optimize_asm(asm, 2), asm);
    }

    #[test]
    fn store_aborts_load_window() {
        let asm = "    mov -8(%rbp), %rax\n    mov %rcx, -8(%rbp)\n    mov -8(%rbp), %rax\n";
        assert_eq!(optimize_asm(asm, 2), asm);
    }

    #[test]
    fn source_register_clobber_aborts_window() {
        // The fib miscompile: `movq %rbx, %xmm1`, then `movabs` rewrites
        // %rbx, then the "same" load reads a different value — kept.
        let asm =
            "    movq %rbx, %xmm1\n    movabs $4598175219545276416, %rbx\n    movq %rbx, %xmm1\n";
        assert_eq!(optimize_asm(asm, 2), asm);
    }

    #[test]
    fn source_register_kept_allows_elim() {
        // Same shape but the source register is untouched: the duplicate
        // goes away.
        let asm = "    movq %rbx, %xmm1\n    movabs $1, %rcx\n    movq %rbx, %xmm1\n";
        let out = optimize_asm(asm, 2);
        assert_eq!(out, "    movq %rbx, %xmm1\n    movabs $1, %rcx\n");
    }

    #[test]
    fn implicit_writes_abort_window() {
        let asm = "    mov -8(%rbp), %rax\n    mulq %rbx\n    mov -8(%rbp), %rax\n";
        assert_eq!(optimize_asm(asm, 2), asm);
    }

    #[test]
    fn passes_for_tiers() {
        assert!(passes_for(0).is_empty());
        assert_eq!(passes_for(1), &["o1"]);
        assert_eq!(passes_for(2), &["o1", "o2"]);
        assert_eq!(passes_for(3), &["o1", "o2"]);
    }
}
