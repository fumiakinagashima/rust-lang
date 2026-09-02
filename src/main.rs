mod lexer;

use std::env;
use std::fs;
use std::process::ExitCode;

use lexer::Lexer;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: na <script.na>");
        return ExitCode::FAILURE;
    };

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("na: failed to read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    match Lexer::new(&source).tokenize() {
        Ok(tokens) => {
            for token in tokens {
                println!("{token:?}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("na: lex error ({}:{}): {}", err.line, err.column, err.message);
            return ExitCode::FAILURE
        }
    }
}
