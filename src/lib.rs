pub mod ast;
pub mod cli;
pub mod codegen;
pub mod diagnostics;
pub mod driver;
pub mod lexer;
pub mod parser;
pub mod tools;

const STACK_SIZE: usize = 16 * 1024 * 1024; // 16 MB stack for compiler traversal

pub fn run_cli() {
    driver::init_console();
    let args = cli::CliArgs::parse();

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
