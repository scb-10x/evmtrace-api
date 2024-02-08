use axum::{
    extract::{Path, State},
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
    let chain_id = chain_id.parse::<i64>()?;

    let results = postgres
        .query(
            "SELECT from_address, to_address, transaction_hash, transaction_index, block_number, value, input, gas_used_total, error, function_signature FROM transactions WHERE chain_id = $1 AND transaction_hash = $2 LIMIT 1",
            &[&chain_id, &hash],
        )
        .await?;
    let result = results.get(0).ok_or_else(AppError::not_found)?;

    Ok(Json(json!({
        "chain_id": chain_id,
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
            "function_signature": result.try_get::<_, Option<String>>("function_signature")?,
        }
    })))
}
