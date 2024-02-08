use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::get,
    Json, Router,
};
use serde_json::{from_str, json, Number, Value};

use crate::{
    error::AppError,
    middleware::DefaultAlwaysCacheMiddleware,
    state::{State as AppState, STATE},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:chain-id/:hash", get(tx_hash))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            DefaultAlwaysCacheMiddleware::handler,
        ))
}

pub async fn tx_hash(
    Path((chain_id, hash)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;

    let result = postgres
        .query_one(
            "SELECT from_address, to_address, transaction_hash, transaction_index, block_number, value, input, gas_used_total, error FROM transactions WHERE chain_id = $1 AND transaction_hash = $2 LIMIT 1",
            &[&chain_id.parse::<i64>()?, &hash],
        )
        .await.map_err(|e| AppError::status(StatusCode::NOT_FOUND, anyhow!(e)))?;

    Ok(Json(json!({
        "data": {
            "from_address": result.try_get::<_, String>("from_address")?,
            "to_address": result.try_get::<_, String>("to_address")?,
            "transaction_hash": result.try_get::<_, String>("transaction_hash")?,
            "transaction_index": result.try_get::<_, i32>("transaction_index")?,
            "block_number": result.try_get::<_, i64>("block_number")?,
            "value": from_str::<Number>(&result.try_get::<_, String>("value")?)?,
            "input": result.try_get::<_, String>("input")?,
            "gas_used_total": result.try_get::<_, i64>("gas_used_total")?,
            "error": result.try_get::<_, Option<String>>("error")?,
        }
    })))
}
