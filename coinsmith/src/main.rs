use serde::Serialize;
use std::env;
use std::error::Error;
use std::fs;
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Fatal error: {}", e);
        process::exit(1);
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorObj {
    ok: bool,
    error: ErrorMsg,
}

#[derive(Debug, Serialize)]
pub struct ErrorMsg {
    code: String,
    message: String,
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: cli <input_fixture.json> <output.json>");
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let fixture_raw = fs::read_to_string(input_path)?;

    let result_json = match coinsmith::run(&fixture_raw) {
        Ok(success) => serde_json::to_string_pretty(&success)?,
        Err((code, message)) => {
            let error_obj = ErrorObj {
                ok: false,
                error: { ErrorMsg { code, message } },
            };
            serde_json::to_string_pretty(&error_obj)?
        }
    };

    fs::write(output_path, result_json)?;

    Ok(())
}
