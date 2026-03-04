use axum::{Json, http::StatusCode};
use coinsmith::input_validation::types::RawFixture;
use coinsmith::run;

pub async fn build_tx(
    Json(fixture): Json<RawFixture>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let raw = serde_json::to_string(&fixture).unwrap();

    match run(&raw) {
        Ok(result) => Ok(Json(serde_json::to_value(result).unwrap())),

        Err((code, message)) => {
            let error = serde_json::json!({
                "ok": false,
                "error": {
                    "code": code,
                    "message": message
                }
            });

            Ok(Json(error))
        }
    }
}
