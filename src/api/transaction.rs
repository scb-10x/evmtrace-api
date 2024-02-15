use axum::{
    extract::{Path, State},
    middleware,
    routing::get,
    Json, Router,
};
use serde_json::{from_str, json, Number, Value};

use crate::{
    error::AppError,
    middleware::LongAlwaysCacheMiddleware,
    state::{State as AppState, STATE},
};

pub fn routes() -> Router<()> {
    Router::new()
        .route("/:hash", get(tx_hash))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            LongAlwaysCacheMiddleware::<false>::handler,
        ))
        .with_state(STATE.clone())
}

pub async fn tx_hash(
    Path(hash): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;

    let results = postgres
        .query(
            "SELECT transactions.chain_id, from_address, to_address, transaction_hash, transaction_index, block_number, blocks.timestamp AS block_timestamp, value, input, gas_used_total, gas_used_first_degree, error, function_signature, sig_names.name AS function_name, ec_pairing_count, ec_recover_count, ec_recover_addresses, ec_pairing_input_sizes, closest_address FROM transactions LEFT JOIN sig_names ON transactions.function_signature = sig_names.sig LEFT JOIN blocks ON blocks.chain_id = transactions.chain_id AND blocks.number = transactions.block_number WHERE transaction_hash = $1 LIMIT 1",
            &[&hash],
        )
        .await?;
    let result = results.get(0).ok_or_else(AppError::not_found)?;

    Ok(Json(json!({
        "data": {
            "chain_id": result.try_get::<_, i64>("chain_id")?,
            "from_address": result.try_get::<_, String>("from_address")?,
            "to_address": result.try_get::<_, String>("to_address")?,
            "transaction_hash": result.try_get::<_, String>("transaction_hash")?,
            "transaction_index": result.try_get::<_, i32>("transaction_index")?,
            "block_number": result.try_get::<_, i64>("block_number")?,
            "block_timestamp": result.try_get::<_, i64>("block_timestamp")?,
            "value": from_str::<Number>(&result.try_get::<_, String>("value")?)?,
            "input": result.try_get::<_, String>("input")?,
            "gas_used_total": result.try_get::<_, i64>("gas_used_total")?,
            "gas_used_first_degree": result.try_get::<_, i64>("gas_used_first_degree")?,
            "error": result.try_get::<_, Option<String>>("error")?,
            "function_signature": result.try_get::<_, Option<String>>("function_signature")?,
            "function_name": result.try_get::<_, Option<String>>("function_name")?,
            "ec_pairing_count": result.try_get::<_, i16>("ec_pairing_count")?,
            "ec_recover_count": result.try_get::<_, i16>("ec_recover_count")?,
            "ec_recover_addresses": result.try_get::<_, Vec<String>>("ec_recover_addresses")?,
            "ec_pairing_input_sizes": result.try_get::<_, Vec<i32>>("ec_pairing_input_sizes")?,
            "closest_address": result.try_get::<_, Vec<String>>("closest_address")?,
        }
    })))
}
