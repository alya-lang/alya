use alya::cli::CliArgs;
use alya::driver;

const STACK_SIZE: usize = 16 * 1024 * 1024; // 16 MB stack for compiler traversal

fn main() {
    driver::init_console();
    let args = CliArgs::parse();

    let child = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || driver::run(args))
        .expect("Failed to spawn compiler driver thread");

    match child.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
        Err(panic_payload) => {
            std::panic::resume_unwind(panic_payload);
        }
    }
}
