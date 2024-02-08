use axum::{
    extract::{Path, State},
    middleware,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    middleware::DefaultAlwaysCacheMiddleware,
    state::{State as AppState, STATE},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:chain-id/:block-number", get(block))
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

    let result = postgres
        .query_one(
            "SELECT number, timestamp, hash, parent_hash, transaction_count, nonce, miner, difficulty, total_difficulty, size, gas_limit, gas_used, base_fee_per_gas FROM blocks WHERE chain_id = $1 AND number = $2 LIMIT 1",
            &[&chain_id, &block_number.parse::<i64>()?],
        )
        .await?;

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
