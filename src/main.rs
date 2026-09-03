mod lexer;
mod parser;

use std::env;
use std::fs;
use std::process::ExitCode;

use lexer::Lexer;
use parser::Parser;


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

    let tokens = match Lexer::new(&source).tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("na: lex error ({}:{}): {}", err.line, err.column, err.message);
            return ExitCode::FAILURE
        }
    };

    match Parser::new(tokens).parse_program() {
        Ok(program) => {
            for stmt in program {
                println!("{stmt:#?}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("na parse error ({}:{}): {}", err.line, err.column, err.message);
            ExitCode::FAILURE
        }
    }
}
