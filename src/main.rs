use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match args.get(1) {
        Some(path) => {
            println!("na: {path} を実行します（未実装）");
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("usage: na <script.na>");
            ExitCode::FAILURE
        }
    }
}
