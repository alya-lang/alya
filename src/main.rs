use alya::cli::CliArgs;
use alya::driver;

fn main() {
    driver::init_console();
    let args = CliArgs::parse();
    if let Err(err) = driver::run(args) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
