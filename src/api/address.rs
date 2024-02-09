use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use ethers_core::{types::Address, utils::to_checksum};
use serde_json::{from_str, json, Number, Value};

use crate::{
    error::AppError,
    middleware::ShortAlwaysCacheMiddleware,
    state::{AppState, STATE},
    types::Pagination,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:chain-id/:address", get(address))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            ShortAlwaysCacheMiddleware::<true>::handler,
        ))
}

pub async fn address(
    Path((chain_id, address)): Path<(String, String)>,
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let chain_id = chain_id.parse::<i64>()?;
    let address = to_checksum(&Address::from_str(&address)?, None);

    let results = postgres
        .query(
            "
                WITH tb AS (
                	SELECT *, 'recovered' AS type FROM transactions WHERE chain_id = $1 AND $2 = ANY(ec_recover_addresses)
                	UNION ALL
                	SELECT *, 'from' AS type FROM transactions WHERE chain_id = $1 AND from_address = $2
                	UNION ALL
                	SELECT *, 'to' AS type FROM transactions WHERE chain_id = $1 AND to_address = $2
                )
                SELECT from_address, to_address, transaction_hash, transaction_index, value, gas_used_total, error, function_signature, block_number, type FROM tb ORDER BY id DESC OFFSET $3 LIMIT $4
            ",
            &[
                &chain_id,
                &address,
                &pagination.offset(),
                &pagination.limit(),
            ],
        )
        .await?;

    let datas = results
        .iter()
        .map(|result| {
            Ok(json!({
                "type": result.try_get::<_, String>("type")?,
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
        "address": address,
        "pagination": pagination,
        "data": datas,
    })))
}
