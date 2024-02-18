use anyhow::Error;
use axum::{
    extract::{Path, State},
    middleware,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    middleware::ShortAlwaysCacheMiddleware,
    state::{AppState, STATE},
};

pub fn routes() -> Router<()> {
    Router::new()
        .route("/:address", get(tag_address))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            ShortAlwaysCacheMiddleware::<false>::handler,
        ))
        .with_state(STATE.clone())
}

pub async fn tag_address(
    Path(address): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;
    let mut address_list = address.split(",").collect::<Vec<&str>>();
    address_list.truncate(20);

    let results = postgres
        .query(
            "SELECT address, ARRAY_AGG(tag) as tags FROM tags WHERE address = ANY($1) GROUP BY address",
            &[&address_list],
        )
        .await?;

    let data = results
        .into_iter()
        .map(|row| {
            Ok::<_, Error>(json!({
                "address": row.try_get::<_, String>("address")?,
                "tags": row.try_get::<_, Vec<String>>("tags")?,
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!({ "data": data })))
}
