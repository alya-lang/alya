pub fn print_version() {
    println!(
        "alya {} ({}-{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

pub fn print_usage() {
    println!(
        "Alya Language Toolchain (alya) v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("A modern, simple, compiled systems programming language.\n");
    println!("USAGE:");
    println!("  alya [COMMAND] [file] [OPTIONS]");
    println!("  alya                                 # Starts interactive REPL\n");
    println!("COMMANDS:");
    println!("  repl                  Start interactive REPL console (default when no args)");
    println!(
        "  run [file]            Compile and execute program immediately (auto-detects entry)"
    );
    println!("  build [file]          Compile program directly to an executable binary (-b, -c)");
    println!("  check [file]          Verify syntax and structure without generating code");
    println!("  ast <file>            Print the parsed Abstract Syntax Tree (AST)");
    println!("  tokens <file>         Print tokenized output from lexical analysis");
    println!("  fmt [path]            Format Alya source code in-place (--check to verify)");
    println!("  test [path]           Discover and run Alya test suites");
    println!("  init [path]           Initialize a new Alya package (creates alya.toml)");
    println!(
        "  add <name>            Add a dependency to alya.toml (--path, --git, --tag, --branch)"
    );
    println!("  install               Resolve and lock all dependencies in alya.lock");
    println!(
        "  update [-u]           Check or upgrade dependencies (-u updates alya.toml & locks)"
    );
    println!("  outdated              Check for newer dependency versions without upgrading");
    println!("  pkg <cmd>             Package manager commands (init, add, install, update, cache, clean)");
    println!("  toolchain <cmd>       Manage C/Assembly build toolchains (status, install, clean)");
    println!("  lsp                   Start Language Server Protocol (LSP) over stdio");
    println!("  doc [path]            Generate HTML and Markdown API documentation");
    println!("  lint [path]           Run static code linter (--fix to auto-refactor, --check for CI)");
    println!("  help                  Display help information");
    println!("  version               Display version information\n");
    println!("OPTIONS:");
    println!("  -o, --output <file>   Specify output file (default: <name>.s or <name>.exe)");
    println!("  -b, -c, --binary      Compile directly to executable (calls GCC)");
    println!("  -r, --run             Compile and run immediately");
    println!("  -S, --asm             Emit assembly output only");
    println!("  --bundle, --app       Package output into a macOS .app Application Bundle");
    println!("  --bundle-id <id>      Set CFBundleIdentifier (default: com.alya.<name>)");
    println!("  --icon <path>         Set custom application icon (.icns) for macOS bundle");
    println!("  --fix                 Automatically apply quick fixes (with lint)");
    println!("  --check               Check without modifying (with fmt or lint)");
    println!("  -u, --upgrade         Rewrite alya.toml with latest versions (with update)");
    println!(
        "  --arch <arch>         Target architecture: x86, x64, arm64 (default: auto-detected)"
    );
    println!("  --os <os>             Target OS: windows, linux, macos (default: auto-detected)");
    println!("  -j, --jobs <N>        Number of parallel test worker jobs (default: CPU cores)");
    println!("  --sequential          Run tests sequentially in single thread (alias for -j 1)");
    println!("  -q, --quiet           Suppress status messages and compiler banner");
    println!("  --time                Display timing for each compilation phase");
    println!(
        "  --no-std              Bare-metal mode: detach standard library and omit libc stubs"
    );
    println!(
        "  --mem-trace           Enable memory diagnostics, allocation tracking, and leak detection"
    );
    println!("  --stats, --bench      Display detailed compilation and execution metrics");
    println!("  -v, --version         Show compiler version");
    println!("  -h, --help            Show this help message\n");
    println!("EXAMPLES:");
    println!("  alya                                 # Start interactive REPL");
    println!("  alya repl                            # Start interactive REPL");
    println!("  alya init my_app                     # Initialize a new package");
    println!("  alya add raylib --path ../raylib     # Add local path dependency");
    println!("  alya install                         # Lock and install dependencies");
    println!("  alya update                          # Check for newer package versions");
    println!("  alya update -u                       # Upgrade alya.toml and re-lock");
    println!("  alya run                             # Run package entry from alya.toml");
    println!("  alya run hello.alya                  # Compile & run in one step");
    println!("  alya build hello.alya                # Produce executable (hello.exe / hello)");
    println!("  alya build app.alya --bundle         # Produce macOS Application Bundle (app.app)");
    println!("  alya hello.alya                      # Produce assembly (hello.s)");
    println!("  alya hello.alya -b -o my_app.exe     # Produce custom named binary");
    println!("  alya fmt hello.alya                  # Format single file");
    println!("  alya fmt . --check                   # Check formatting for entire codebase");
    println!("  alya test                            # Run all tests in project");
    println!("  alya check hello.alya                # Quick syntax validation");
    println!("  alya ast hello.alya                  # Inspect AST hierarchy");
    println!("  alya pkg list                        # List dependencies and lock status");
    println!("  alya pkg cache                       # Inspect package cache and storage");
    println!(
        "  alya pkg clean                       # Clean cached packages and reclaim disk space"
    );
}
