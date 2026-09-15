pub fn print_version() {
    println!(
        "alyac {} ({}-{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

pub fn print_usage() {
    println!(
        "Alya Programming Language Compiler (alyac) v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("A modern, simple, compiled programming language.\n");
    println!("USAGE:");
    println!("  alyac [COMMAND] [file] [OPTIONS]");
    println!("  alyac                                # Starts interactive REPL\n");
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
    println!("  pkg <cmd>             Package manager commands (init, add, install, list, update)");
    println!("  toolchain <cmd>       Manage C/Assembly build toolchains (status, install, clean)");
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
    println!("  --check               Check formatting without modifying (with fmt)");
    println!(
        "  --arch <arch>         Target architecture: x86, x64, arm64 (default: auto-detected)"
    );
    println!("  --os <os>             Target OS: windows, linux, macos (default: auto-detected)");
    println!("  -j, --jobs <N>        Number of parallel test worker jobs (default: CPU cores)");
    println!("  --sequential          Run tests sequentially in single thread (alias for -j 1)");
    println!("  -q, --quiet           Suppress status messages and compiler banner");
    println!("  --time                Display timing for each compilation phase");
    println!("  --stats, --bench      Display detailed compilation and execution metrics");
    println!("  -v, --version         Show compiler version");
    println!("  -h, --help            Show this help message\n");
    println!("EXAMPLES:");
    println!("  alyac                                # Start interactive REPL");
    println!("  alyac repl                           # Start interactive REPL");
    println!("  alyac init my_app                    # Initialize a new package");
    println!("  alyac add raylib --path ../raylib    # Add local path dependency");
    println!("  alyac install                        # Lock and install dependencies");
    println!("  alyac run                            # Run package entry from alya.toml");
    println!("  alyac run hello.alya                 # Compile & run in one step");
    println!("  alyac build hello.alya               # Produce executable (hello.exe / hello)");
    println!("  alyac build app.alya --bundle        # Produce macOS Application Bundle (app.app)");
    println!("  alyac hello.alya                     # Produce assembly (hello.s)");
    println!("  alyac hello.alya -b -o my_app.exe    # Produce custom named binary");
    println!("  alyac fmt hello.alya                 # Format single file");
    println!("  alyac fmt . --check                  # Check formatting for entire codebase");
    println!("  alyac test                           # Run all tests in project");
    println!("  alyac check hello.alya               # Quick syntax validation");
    println!("  alyac ast hello.alya                 # Inspect AST hierarchy");
    println!("  alyac pkg list                       # List dependencies and lock status");
    println!("  alyac pkg cache                      # Inspect package cache and storage");
    println!(
        "  alyac pkg clean                      # Clean cached packages and reclaim disk space"
    );
}
