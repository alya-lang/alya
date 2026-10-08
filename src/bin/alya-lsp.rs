fn print_help() {
    println!("alya-lsp - Standalone Alya Language Server Protocol server over stdio\n");
    println!("Usage:");
    println!(
        "  alya-lsp                  # Start LSP server (no arguments; editors spawn it directly)"
    );
    println!("  alya-lsp --help           # Show this help");
    println!("  alya-lsp --version        # Show version\n");
    println!("Equivalent to `alya lsp`, kept as an alias for backward compatibility.");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("alya-lsp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    alya::run_lsp();
}
