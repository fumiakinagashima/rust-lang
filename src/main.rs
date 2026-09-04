mod lexer;
mod parser;
mod typecheck;

use std::env;
use std::fs;
use std::process::ExitCode;

use lexer::Lexer;
use parser::Parser;
use typecheck::TypeChecker;

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

    let program = match Parser::new(tokens).parse_program() {
        Ok(program) => program,
        Err(err) => {
            eprintln!("na parse error ({}:{}): {}", err.line, err.column, err.message);
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = TypeChecker::new().check_program(&program) {
        eprintln!("na: type error ({}:{}): {}", err.line, err.column, err.message);
        return ExitCode::FAILURE;
    }

    for stmt in program {
        println!("{stmt:#?}");
    }
    ExitCode::SUCCESS
}
