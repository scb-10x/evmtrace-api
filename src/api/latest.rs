use axum::{extract::State, middleware, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    middleware::ShortAlwaysCacheMiddleware,
    state::{AppState, STATE},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/blocks", get(latest_block))
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
            SELECT chain_id, number, timestamp, hash, transaction_count, COALESCE(txs.rtc, 0) AS related_transaction_count FROM blocks LEFT JOIN txs ON blocks.chain_id = txs.chain_id AND blocks.number = txs.block_number ORDER BY id DESC LIMIT 20
            ",
            &[],
        )
        .await?;
    let datas = results
        .iter()
        .map(|row| {
            Ok(json!({
                "chain_id": row.try_get::<_, String>("chain_id")?,
                "number": row.try_get::<_, i64>("number")?,
                "timestamp": row.try_get::<_, i64>("timestamp")?,
                "hash": row.try_get::<_, String>("hash")?,
                "transaction_count": row.try_get::<_, i64>("transaction_count")?,
                "related_transaction_count": row.try_get::<_, i64>("related_transaction_count")?,
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(json!({
        "data": datas,
    })))
}
