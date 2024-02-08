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
        .route("/:chain-id/:block-number", get(block))
        .route("/:chain-id/:block-number/txs", get(block_txs))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            DefaultAlwaysCacheMiddleware::handler,
        ))
}

pub async fn block(
    Path((chain_id, block_number)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let chain_id = chain_id.parse::<i64>()?;
    let block_number = block_number.parse::<i64>()?;

    let results = postgres
        .query(
            "SELECT number, timestamp, hash, parent_hash, transaction_count, nonce, miner, difficulty, total_difficulty, size, gas_limit, gas_used, base_fee_per_gas FROM blocks WHERE chain_id = $1 AND number = $2 LIMIT 1",
            &[&chain_id, &block_number],
        )
        .await?;
    let result = results.get(0).ok_or_else(AppError::not_found)?;

    Ok(Json(json!({
        "chain_id": chain_id,
        "data": {
            "number": result.try_get::<_, i64>("number")?,
            "timestamp": result.try_get::<_, i64>("timestamp")?,
            "hash": result.try_get::<_, String>("hash")?,
            "parent_hash": result.try_get::<_, String>("parent_hash")?,
            "transaction_count": result.try_get::<_, i32>("transaction_count")?,
            "nonce": result.try_get::<_, String>("nonce")?,
            "miner": result.try_get::<_, String>("miner")?,
            "difficulty": result.try_get::<_, i64>("difficulty")?,
            "total_difficulty": result.try_get::<_, f64>("total_difficulty")?,
            "size": result.try_get::<_, i32>("size")?,
            "gas_limit": result.try_get::<_, i64>("gas_limit")?,
            "gas_used": result.try_get::<_, i64>("gas_used")?,
            "base_fee_per_gas": result.try_get::<_, i64>("base_fee_per_gas")?,
        },
    })))
}

pub async fn block_txs(
    Path((chain_id, block_number)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let chain_id = chain_id.parse::<i64>()?;
    let block_number = block_number.parse::<i64>()?;

    let results = postgres
        .query(
            "SELECT from_address, to_address, transaction_hash, transaction_index, value, gas_used_total, error, function_signature, block_number FROM transactions WHERE chain_id = $1 AND block_number = $2",
            &[&chain_id, &block_number],
        )
        .await?;
    let datas = results
        .iter()
        .map(|result| {
            Ok(json!({
                "from_address": result.try_get::<_, String>("from_address")?,
                "to_address": result.try_get::<_, String>("to_address")?,
                "transaction_hash": result.try_get::<_, String>("transaction_hash")?,
                "transaction_index": result.try_get::<_, i32>("transaction_index")?,
                "value": from_str::<Number>(&result.try_get::<_, String>("value")?)?,
                "gas_used_total": result.try_get::<_, i64>("gas_used_total")?,
                "error": result.try_get::<_, Option<String>>("error")?,
                "function_signature": result.try_get::<_, String>("function_signature")?,
                "block_number": result.try_get::<_, i64>("block_number")?,
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(json!({
        "chain_id": chain_id,
        "block_number": block_number,
        "data": datas
    })))
}
