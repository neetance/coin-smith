/*
 This is the main entry point of the project, where we will read the input fixture from a file, call the main function
 from the lib.rs file to get the psbt result, and then write the result to an output file. We will also handle any errors
 that occur during the process.
*/

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
    // getting the command line arguments for the input fixture file and the output file, and if the arguments are
    // not provided correctly, we print the usage and exit with an error code.
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: cli <input_fixture.json> <output.json>");
        process::exit(1);
    }

    // we get the input and output file paths from the command line arguments
    let input_path = &args[1];
    let output_path = &args[2];

    // we read the input fixture from the specified file, and if there is an error during reading the file, we return
    // an error with the appropriate message.
    let fixture_raw = fs::read_to_string(input_path)?;

    // we call the main function from the lib.rs file to get the psbt result, and if there is an error during the process,
    // we create an error object with the appropriate code and message, and serialize it to JSON format to be written to the
    // output file.
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
