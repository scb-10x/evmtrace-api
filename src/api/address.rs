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

pub fn routes() -> Router<()> {
    Router::new()
        .route("/:address", get(address))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            ShortAlwaysCacheMiddleware::<true>::handler,
        ))
        .with_state(STATE.clone())
}

pub async fn address(
    Path(address): Path<String>,
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let address = to_checksum(&Address::from_str(&address)?, None);

    let results = postgres
        .query(
            "
                WITH tb AS (
                	SELECT * FROM transactions WHERE from_address = $1 OR to_address = $1 OR ARRAY[$1]::VARCHAR[] <@ ec_recover_addresses OR ARRAY[$1]::VARCHAR[] <@ closest_address
                )
                SELECT chain_id, from_address, to_address, closest_address, transaction_hash, transaction_index, value, error, function_signature, sig_names.name AS function_name, block_number, ec_pairing_count, ec_recover_addresses FROM tb LEFT JOIN sig_names ON tb.function_signature = sig_names.sig ORDER BY tb.id DESC OFFSET $2 LIMIT $3
            ",
            &[
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
                "chain_id": result.try_get::<_, i64>("chain_id")?,
                "from_address": result.try_get::<_, String>("from_address")?,
                "to_address": result.try_get::<_, String>("to_address")?,
                "closest_address": result.try_get::<_, Vec<String>>("closest_address")?,
                "transaction_hash": result.try_get::<_, String>("transaction_hash")?,
                "transaction_index": result.try_get::<_, i32>("transaction_index")?,
                "value": from_str::<Number>(&result.try_get::<_, String>("value")?)?,
                "error": result.try_get::<_, Option<String>>("error")?,
                "function_signature": result.try_get::<_, String>("function_signature")?,
                "function_name": result.try_get::<_, Option<String>>("function_name")?,
                "block_number": result.try_get::<_, i64>("block_number")?,
                "ec_pairing_count": result.try_get::<_, i16>("ec_pairing_count")?,
                "ec_recover_addresses": result.try_get::<_, Vec<String>>("ec_recover_addresses")?,
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(json!({
        "address": address,
        "pagination": pagination,
        "data": datas,
    })))
}
