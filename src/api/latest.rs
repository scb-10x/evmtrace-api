use axum::{extract::State, middleware, routing::get, Json, Router};
use serde_json::{from_str, json, Number, Value};

use crate::{
    error::AppError,
    middleware::ShortAlwaysCacheMiddleware,
    state::{AppState, STATE},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/blocks", get(latest_block))
        .route("/txs", get(latest_txs))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            ShortAlwaysCacheMiddleware::<false>::handler,
        ))
}

pub async fn latest_block(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let results = postgres
        .query(
            "
            WITH latest_txs AS (
            	SELECT chain_id, block_number FROM transactions ORDER BY id DESC LIMIT 1000
            ),
            txs AS (
            	SELECT *, COUNT(*) AS rtc FROM latest_txs GROUP BY chain_id, block_number
            )
            SELECT blocks.chain_id, number, timestamp, hash, transaction_count, COALESCE(txs.rtc, 0) AS related_transaction_count FROM blocks LEFT JOIN txs ON blocks.chain_id = txs.chain_id AND blocks.number = txs.block_number ORDER BY id DESC LIMIT 20
            ",
            &[],
        )
        .await?;
    let datas = results
        .iter()
        .map(|row| {
            Ok(json!({
                "chain_id": row.try_get::<_, i64>("chain_id")?,
                "number": row.try_get::<_, i64>("number")?,
                "timestamp": row.try_get::<_, i64>("timestamp")?,
                "hash": row.try_get::<_, String>("hash")?,
                "transaction_count": row.try_get::<_, i32>("transaction_count")?,
                "related_transaction_count": row.try_get::<_, i64>("related_transaction_count")?,
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(json!({
        "data": datas,
    })))
}

pub async fn latest_txs(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let results = postgres
        .query(
            "
            WITH lb AS (
            	SELECT chain_id, number, timestamp FROM blocks ORDER BY id DESC LIMIT 100
            ),
            ltxs AS (SELECT lb.chain_id, lb.number, lb.timestamp, transaction_hash, from_address, to_address, value, error FROM transactions INNER JOIN lb ON lb.chain_id = transactions.chain_id AND lb.number = transactions.block_number ORDER BY id DESC LIMIT 50)
            SELECT * FROM ltxs ORDER BY timestamp DESC
            ",
            &[],
        )
        .await?;
    let datas = results
        .iter()
        .map(|row| {
            Ok(json!({
                "chain_id": row.try_get::<_, i64>("chain_id")?,
                "number": row.try_get::<_, i64>("number")?,
                "timestamp": row.try_get::<_, i64>("timestamp")?,
                "transaction_hash": row.try_get::<_, String>("transaction_hash")?,
                "from_address": row.try_get::<_, String>("from_address")?,
                "to_address": row.try_get::<_, String>("to_address")?,
                "value": from_str::<Number>(&row.try_get::<_, String>("value")?)?,
                "error": row.try_get::<_, Option<String>>("error")?
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(json!({
        "data": datas,
    })))
}
