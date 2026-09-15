use crate::codegen::{Architecture, OperatingSystem};
use std::io::{self, Write};

pub mod analyzer;
pub mod executor;
pub mod session;

#[cfg(test)]
mod tests;

pub use analyzer::*;
pub use executor::*;
pub use session::*;

fn print_repl_banner() {
    const INNER_WIDTH: usize = 56;
    let border = "═".repeat(INNER_WIDTH);
    println!("\x1b[1;36m╔{}╗\x1b[0m", border);
    crate::driver::console::print_box_row("              \x1b[1;33m⚡ ALYA INTERACTIVE REPL ⚡\x1b[0m", INNER_WIDTH);
    crate::driver::console::print_box_row(&format!(
        "  \x1b[0mVersion {} ({}-{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    ), INNER_WIDTH);
    crate::driver::console::print_box_row("  \x1b[0mType \x1b[1;32m:help\x1b[0m for commands, \x1b[1;31m:exit\x1b[0m to quit", INNER_WIDTH);
    println!("\x1b[1;36m╚{}╝\x1b[0m\n", border);
}

/// Starts the interactive REPL read-eval-print loop.
pub fn start_repl(arch: Architecture, os: OperatingSystem) -> Result<(), String> {
    crate::driver::init_console();
    print_repl_banner();

    let mut session = ReplSession::new(arch, os);
    let mut input_buffer = String::new();

    let stdin = io::stdin();

    loop {
        if input_buffer.is_empty() {
            print!("\x1b[1;32malya>\x1b[0m ");
        } else {
            print!("\x1b[1;34m ...>\x1b[0m ");
        }
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut line = String::new();
        let bytes_read = stdin.read_line(&mut line).map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            // EOF reached (Ctrl+D / Ctrl+Z)
            println!("\n\x1b[1;33mGoodbye!\x1b[0m");
            break;
        }

        let trimmed_line = line.trim();

        // Allow cancelling incomplete multiline inputs with :reset, :cancel
        if !input_buffer.is_empty() && (trimmed_line == ":reset" || trimmed_line == ":cancel") {
            input_buffer.clear();
            println!("\x1b[1;33mInput discarded.\x1b[0m");
            continue;
        }

        input_buffer.push_str(&line);

        if !is_input_incomplete(&input_buffer) {
            let full_input = std::mem::take(&mut input_buffer);
            session.eval_input(&full_input);
        }
    }

    Ok(())
}
