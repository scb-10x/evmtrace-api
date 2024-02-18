use std::time::SystemTime;

use axum::{extract::State, middleware, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    middleware::LongAlwaysCacheMiddleware,
    state::{AppState, STATE},
};

pub fn routes() -> Router<()> {
    Router::new()
        .route("/tx_count", get(tx_count))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            LongAlwaysCacheMiddleware::<false>::handler,
        ))
        .with_state(STATE.clone())
}

pub async fn tx_count(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let results = postgres.query("SELECT interval_start AS date, chain_id, transaction_count, total_transaction_count FROM transaction_counts_mv ORDER BY 1 DESC", &[]).await?;
    let data = results
        .iter()
        .map(|result| {
            Ok(json!((
                result
                    .try_get::<_, SystemTime>("date")?
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs(),
                result.try_get::<_, i64>("chain_id")?,
                result.try_get::<_, i64>("transaction_count")?,
                result.try_get::<_, i64>("total_transaction_count")?,
            )))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(json!({
        "data": data,
    })))
}
