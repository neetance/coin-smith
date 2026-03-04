use axum::{
    Json, Router,
    routing::{get, post},
};
use serde::Serialize;
use serde_json::json;
use std::env;
use std::error::Error;
use std::fs;
use std::process;
use tower_http::cors::CorsLayer;

mod api;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && args[1] == "server" {
        start_server().await;
        return;
    }

    if let Err(e) = run_cli(&args) {
        eprintln!("Fatal error: {}", e);
        process::exit(1);
    }
}

async fn start_server() {
    let app = Router::new()
        .route("/api/build", post(api::build_tx))
        .route("/api/health", get(health))
        .layer(CorsLayer::permissive());

    println!("Server running on http://localhost:8080");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true
    }))
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

fn run_cli(args: &[String]) -> Result<(), Box<dyn Error>> {
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
                error: ErrorMsg { code, message },
            };

            serde_json::to_string_pretty(&error_obj)?
        }
    };

    fs::write(output_path, result_json)?;

    Ok(())
}
