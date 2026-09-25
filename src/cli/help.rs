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
    println!("CORE COMMANDS:");
    println!("  repl                  Start interactive REPL console (default when no args)");
    println!("  run [file]            Compile and execute program immediately");
    println!("  build [file]          Compile program to an executable binary");
    println!("  check [file]          Verify syntax and structure without generating code");
    println!("  fmt [path]            Format source code in-place (--check to verify)");
    println!("  test [path]           Discover and run test suites");
    println!("  bench [path]          Discover and run benchmarks");
    println!("  lint [path]           Run static code linter (--fix, --check)");
    println!("  doc [path]            Generate API documentation");
    println!("  pkg <cmd>             Manage dependencies (init, add, install, update, ...)");
    println!("  help [cmd]            Show help for a command");
    println!("  version               Display version information\n");
    println!("Run 'alya <command> --help' for command details.");
    println!("Run 'alya help --all' for the full reference.");
}

pub fn print_full_usage() {
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
    println!("  bench [path]          Discover and run Alya benchmarks");
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
    println!(
        "  lint [path]           Run static code linter (--fix to auto-refactor, --check for CI)"
    );
    println!("  help                  Display help information");
    println!("  version               Display version information\n");
    println!("OPTIONS:");
    println!("  -o, --output <file>   Specify output file (default: <name>.s or <name>.exe)");
    println!("  -b, -c, --binary      Compile directly to executable (calls GCC)");
    println!("  -r, --run             Compile and run immediately");
    println!("  -S, --asm             Emit assembly output only");
    println!("  --bundle, --app       Package output into a platform bundle (.app on macOS, exe+manifest on Windows, binary+.desktop on Linux)");
    println!("  --gui                 GUI application bundle (implies --bundle; enables DPI awareness and desktop integration)");
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
    println!(
        "  alya build app.alya --bundle --gui   # GUI bundle (DPI-aware manifest / desktop entry)"
    );
    println!("  alya build app.alya --bundle --os windows  # Windows bundle (exe + manifest)");
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

/// Prints focused help for `topic`. Returns true when the topic is known.
pub fn print_command_help(topic: &str) -> bool {
    match topic {
        "repl" => print_repl_help(),
        "run" => print_run_help(),
        "build" => print_build_help(),
        "check" => print_check_help(),
        "ast" => print_ast_help(),
        "tokens" => print_tokens_help(),
        "fmt" => print_fmt_help(),
        "test" => print_test_help(),
        "bench" => print_bench_help(),
        "lint" => print_lint_help(),
        "doc" => print_doc_help(),
        "pkg" => print_pkg_help(),
        "init" => print_init_help(),
        "add" => print_add_help(),
        "install" => print_install_help(),
        "update" => print_update_help(),
        "outdated" => print_outdated_help(),
        "cache" => print_cache_help(),
        "clean" => print_clean_help(),
        "list" => print_list_help(),
        "toolchain" => print_toolchain_help(),
        "lsp" => print_lsp_help(),
        "dap" => print_dap_help(),
        "help" => print_help_help(),
        "version" => print_version_help(),
        _ => return false,
    }
    true
}

pub fn print_repl_help() {
    println!("alya repl - Start interactive REPL console\n");
    println!("USAGE:");
    println!("  alya repl");
    println!("  alya                                 # No args also starts the REPL\n");
    println!("EXAMPLES:");
    println!("  alya");
    println!("  alya repl");
}

pub fn print_run_help() {
    println!("alya run - Compile and execute a program immediately\n");
    println!("USAGE:");
    println!("  alya run [file] [OPTIONS] [-- program args]\n");
    println!("ARGS:");
    println!("  [file]              Source file or package entry (default: alya.toml entry)\n");
    println!("OPTIONS:");
    println!("  --arch <arch>       Target architecture: x86, x64, arm64");
    println!("  --os <os>           Target OS: windows, linux, macos");
    println!("  -q, --quiet         Suppress status messages and compiler banner");
    println!("  --time              Display timing for each compilation phase");
    println!("  --stats, --bench    Display detailed compilation and execution metrics");
    println!("  --no-std            Bare-metal mode: detach standard library");
    println!("  --mem-trace         Enable memory diagnostics and leak detection\n");
    println!("EXAMPLES:");
    println!("  alya run                             # Run package entry from alya.toml");
    println!("  alya run hello.alya                  # Compile & run in one step");
    println!("  alya run app.alya -- --port 8080     # Forward args to the program");
}

pub fn print_build_help() {
    println!("alya build - Compile a program to an executable binary\n");
    println!("USAGE:");
    println!("  alya build [file] [OPTIONS]\n");
    println!("ARGS:");
    println!("  [file]              Source file or package entry (default: alya.toml entry)\n");
    println!("OPTIONS:");
    println!("  -o, --output <file> Specify output file (default: <name>.s or <name>.exe)");
    println!("  -b, -c, --binary    Compile directly to executable (calls GCC)");
    println!("  -S, --asm           Emit assembly output only");
    println!("  -r, --run           Compile and run immediately");
    println!("  --bundle, --app     Package output into a platform bundle");
    println!("  --gui               GUI application bundle (implies --bundle)");
    println!("  --bundle-id <id>    Set CFBundleIdentifier (default: com.alya.<name>)");
    println!("  --icon <path>       Set custom application icon (.icns) for macOS bundle");
    println!("  --arch <arch>       Target architecture: x86, x64, arm64");
    println!("  --os <os>           Target OS: windows, linux, macos");
    println!("  -q, --quiet         Suppress status messages and compiler banner");
    println!("  --time              Display timing for each compilation phase\n");
    println!("EXAMPLES:");
    println!("  alya build hello.alya                # Produce executable (hello.exe / hello)");
    println!("  alya build app.alya --bundle --gui   # GUI bundle for the host platform");
    println!("  alya build app.alya --bundle --os windows  # Cross-platform bundle");
}

pub fn print_check_help() {
    println!("alya check - Verify syntax and structure without generating code\n");
    println!("USAGE:");
    println!("  alya check [file]\n");
    println!("ARGS:");
    println!("  [file]              Source file or package entry (default: alya.toml entry)\n");
    println!("OPTIONS:");
    println!("  -q, --quiet         Suppress status messages and compiler banner\n");
    println!("EXAMPLES:");
    println!("  alya check hello.alya                # Quick syntax validation");
    println!("  alya check                           # Check package entry from alya.toml");
}

pub fn print_ast_help() {
    println!("alya ast - Print the parsed Abstract Syntax Tree (AST)\n");
    println!("USAGE:");
    println!("  alya ast <file>\n");
    println!("ARGS:");
    println!("  <file>              Source file to inspect\n");
    println!("EXAMPLES:");
    println!("  alya ast hello.alya                  # Inspect AST hierarchy");
}

pub fn print_tokens_help() {
    println!("alya tokens - Print tokenized output from lexical analysis\n");
    println!("USAGE:");
    println!("  alya tokens <file>\n");
    println!("ARGS:");
    println!("  <file>              Source file to tokenize\n");
    println!("EXAMPLES:");
    println!("  alya tokens hello.alya               # Inspect lexer output");
}

pub fn print_fmt_help() {
    println!("alya fmt - Format Alya source code in-place\n");
    println!("USAGE:");
    println!("  alya fmt [path] [--check]\n");
    println!("ARGS:");
    println!("  [path]              File or directory to format (default: .)\n");
    println!("OPTIONS:");
    println!("  --check             Check without modifying (CI mode)\n");
    println!("EXAMPLES:");
    println!("  alya fmt hello.alya                  # Format single file");
    println!("  alya fmt . --check                   # Check formatting for entire codebase");
}

pub fn print_test_help() {
    println!("alya test - Discover and run test suites\n");
    println!("USAGE:");
    println!("  alya test [path] [OPTIONS]\n");
    println!("ARGS:");
    println!("  [path]              File or directory to test (default: .)\n");
    println!("OPTIONS:");
    println!("  -j, --jobs <N>      Number of parallel test worker jobs (default: CPU cores)");
    println!("  --sequential        Run tests sequentially (alias for -j 1)\n");
    println!("EXAMPLES:");
    println!("  alya test                            # Run all tests in project");
    println!("  alya test -j 4                       # Run with 4 parallel workers");
    println!("  alya test --sequential               # Run sequentially for debugging");
}

pub fn print_bench_help() {
    println!("alya bench - Discover and run benchmarks\n");
    println!("USAGE:");
    println!("  alya bench [path]\n");
    println!("ARGS:");
    println!("  [path]              File or directory to benchmark (default: .)\n");
    println!("EXAMPLES:");
    println!("  alya bench                           # Run all benchmarks in project");
    println!("  alya bench benches                   # Run benchmarks in a directory");
}

pub fn print_lint_help() {
    println!("alya lint - Run static code linter\n");
    println!("USAGE:");
    println!("  alya lint [path] [OPTIONS]\n");
    println!("ARGS:");
    println!("  [path]              File or directory to lint (default: .)\n");
    println!("OPTIONS:");
    println!("  --fix               Automatically refactor and clean up safe warnings in-place");
    println!("  --check             Exit non-zero on warnings (CI quality gate)\n");
    println!("EXAMPLES:");
    println!("  alya lint                            # Lint the current project");
    println!("  alya lint src --fix                  # Auto-fix safe warnings");
    println!("  alya lint . --check                  # CI gate: fail on warnings");
}

pub fn print_doc_help() {
    println!("alya doc - Generate API documentation\n");
    println!("USAGE:");
    println!("  alya doc [path] [OPTIONS]\n");
    println!("ARGS:");
    println!("  [path]              Source directory to document (default: src or .)\n");
    println!("OPTIONS:");
    println!("  -o, --output <dir>  Output directory (default: docs)");
    println!("  --html              Generate HTML documentation");
    println!("  --md, --markdown    Generate Markdown documentation\n");
    println!("EXAMPLES:");
    println!("  alya doc                             # Document the current project");
    println!("  alya doc src --html                  # HTML docs for src/");
}

pub fn print_pkg_help() {
    println!("Alya Package Manager (alya pkg)");
    println!("Manage project dependencies, manifests (alya.toml), and lockfiles (alya.lock).\n");
    println!("USAGE:");
    println!("  alya pkg <COMMAND> [OPTIONS]");
    println!("  alya init [path] [OPTIONS]          # Shortcut for pkg init");
    println!("  alya add <name> [OPTIONS]           # Shortcut for pkg add");
    println!("  alya install                        # Shortcut for pkg install");
    println!("  alya update [-u | --upgrade]        # Shortcut for pkg update");
    println!("  alya outdated                       # Shortcut for pkg outdated");
    println!("  alya cache                          # Shortcut for pkg cache");
    println!("  alya clean                          # Shortcut for pkg clean\n");
    println!("COMMANDS:");
    println!("  init [path]        Initialize a new Alya package in [path] (default: .)");
    println!("  add <name>         Add a new dependency to alya.toml");
    println!("  install            Resolve and lock all dependencies specified in alya.toml");
    println!("  list               List project dependencies and lock integrity status");
    println!(
        "  update [-u]        Check or upgrade dependencies (-u rewrites alya.toml & re-locks)"
    );
    println!("  outdated           Check for newer versions of dependencies without upgrading");
    println!("  cache [clean]      Inspect package cache directory, size, and installed packages");
    println!("  clean [--all]      Remove cached dependencies and reclaim disk space");
    println!("  help               Show this help message\n");
    println!("OPTIONS FOR 'init':");
    println!("  --name <name>      Override package name (default: directory name)");
    println!("  --lib              Initialize as a library (src/lib.alya) instead of binary\n");
    println!("OPTIONS FOR 'add':");
    println!("  --path <path>      Add dependency from local file system path");
    println!("  --git <url>        Add dependency from remote Git repository");
    println!("  --tag <tag>        Specify Git tag for dependency");
    println!("  --branch <branch>  Specify Git branch for dependency");
    println!("  --version <ver>    Specify semantic version constraint\n");
    println!("OPTIONS FOR 'install':");
    println!("  --strict           Fail instead of falling back to source when a release asset is broken\n");
    println!("OPTIONS FOR 'update':");
    println!(
        "  -u, --upgrade      Rewrite alya.toml with latest versions and re-lock dependencies\n"
    );
    println!("EXAMPLES:");
    println!("  alya init my_app");
    println!("  alya add http                       # Add official package via short-name");
    println!("  alya install                        # Install & lock dependencies");
    println!("  alya update                         # Check for newer package versions");
    println!("  alya update -u                      # Upgrade alya.toml and re-lock");
    println!("  alya pkg outdated                   # Check outdated packages (read-only)");
    println!("  alya pkg cache");
    println!("  alya pkg clean");
}

pub fn print_init_help() {
    println!("alya init - Initialize a new Alya package\n");
    println!("USAGE:");
    println!("  alya init [path] [OPTIONS]\n");
    println!("ARGS:");
    println!("  [path]              Target directory (default: .)\n");
    println!("OPTIONS:");
    println!("  --name <name>       Override package name (default: directory name)");
    println!("  --lib               Initialize as a library (src/lib.alya) instead of binary\n");
    println!("EXAMPLES:");
    println!("  alya init my_app                     # Initialize a new package");
    println!("  alya init my_lib --lib               # Initialize as a library");
}

pub fn print_add_help() {
    println!("alya add - Add a dependency to alya.toml\n");
    println!("USAGE:");
    println!("  alya add <name> [OPTIONS]\n");
    println!("ARGS:");
    println!("  <name>              Package name, 'name@version', or 'owner/repo@tag'\n");
    println!("OPTIONS:");
    println!("  --path <path>       Add dependency from local file system path");
    println!("  --git <url>         Add dependency from remote Git repository");
    println!("  --tag <tag>         Specify Git tag for dependency");
    println!("  --branch <branch>   Specify Git branch for dependency");
    println!("  --version <ver>     Specify semantic version constraint\n");
    println!("EXAMPLES:");
    println!("  alya add http                        # Add official package via short-name");
    println!("  alya add raylib --path ../raylib     # Add local path dependency");
    println!("  alya add http@0.1.0                  # Pin a semantic version");
}

pub fn print_install_help() {
    println!("alya install - Resolve and lock dependencies\n");
    println!("USAGE:");
    println!("  alya install [OPTIONS]\n");
    println!("OPTIONS:");
    println!("  --strict            Fail instead of falling back to source on broken assets\n");
    println!("EXAMPLES:");
    println!("  alya install                         # Install & lock dependencies");
    println!("  alya install --strict                # Strict mode for CI");
}

pub fn print_update_help() {
    println!("alya update - Check or upgrade dependencies\n");
    println!("USAGE:");
    println!("  alya update [-u | --upgrade]\n");
    println!("OPTIONS:");
    println!("  -u, --upgrade       Rewrite alya.toml with latest versions and re-lock\n");
    println!("EXAMPLES:");
    println!("  alya update                          # Check for newer package versions");
    println!("  alya update -u                       # Upgrade alya.toml and re-lock");
}

pub fn print_outdated_help() {
    println!("alya outdated - Check for newer versions without upgrading\n");
    println!("USAGE:");
    println!("  alya outdated\n");
    println!("EXAMPLES:");
    println!("  alya outdated                        # List upgradable dependencies");
    println!("  alya pkg outdated                    # Same via pkg namespace");
}

pub fn print_cache_help() {
    println!("alya cache - Inspect the package cache\n");
    println!("USAGE:");
    println!("  alya cache [clean] [--all]\n");
    println!("EXAMPLES:");
    println!("  alya pkg cache                       # Inspect package cache and storage");
    println!("  alya pkg cache clean --all           # Purge cached packages");
}

pub fn print_clean_help() {
    println!("alya clean - Remove cached dependencies\n");
    println!("USAGE:");
    println!("  alya clean [--all]\n");
    println!("OPTIONS:");
    println!("  --all, -a           Purge the entire global cache\n");
    println!("EXAMPLES:");
    println!("  alya pkg clean                       # Clean cached packages");
    println!("  alya pkg clean --all                 # Reclaim all disk space");
}

pub fn print_list_help() {
    println!("alya pkg list - List dependencies and lock status\n");
    println!("USAGE:");
    println!("  alya pkg list\n");
    println!("EXAMPLES:");
    println!("  alya pkg list                        # List dependencies and lock status");
}

pub fn print_toolchain_help() {
    println!("Alya Toolchain Manager\n");
    println!("USAGE:");
    println!("  alya toolchain <command>\n");
    println!("COMMANDS:");
    println!("  status     Show active compiler, version, and location");
    println!("  install    Pre-emptively download and configure portable toolchain");
    println!("  clean      Purge ~/.alya/toolchain to reclaim disk space");
    println!("  help       Show this help message\n");
    println!("EXAMPLES:");
    println!("  alya toolchain status                # Show active toolchain");
    println!("  alya toolchain install               # Pre-install portable toolchain");
}

pub fn print_lsp_help() {
    println!("alya lsp - Start Language Server Protocol server over stdio\n");
    println!("USAGE:");
    println!("  alya lsp\n");
    println!("EXAMPLES:");
    println!("  alya lsp                             # Start LSP for editor integration");
}

pub fn print_dap_help() {
    println!("alya dap - Start Debug Adapter Protocol server\n");
    println!("USAGE:");
    println!("  alya dap\n");
    println!("EXAMPLES:");
    println!("  alya dap                             # Start DAP server");
}

pub fn print_help_help() {
    println!("alya help - Display help information\n");
    println!("USAGE:");
    println!("  alya help [COMMAND]");
    println!("  alya <command> --help");
    println!("  alya help --all                      # Full command reference\n");
    println!("EXAMPLES:");
    println!("  alya help                            # Short overview");
    println!("  alya help build                      # Focused build usage");
    println!("  alya help --all                      # Full reference");
}

pub fn print_version_help() {
    println!("alya version - Display version information\n");
    println!("USAGE:");
    println!("  alya version");
    println!("  alya -v | --version\n");
    println!("EXAMPLES:");
    println!("  alya version");
}
