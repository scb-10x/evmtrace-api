use anyhow::Error;
use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    middleware::LongAlwaysCacheMiddleware,
    state::{AppState, STATE},
    types::Pagination,
};

pub fn routes() -> Router<()> {
    Router::new()
        .nest(
            "/",
            Router::new()
                .route("/all", get(all_tags))
                .route("/:address", get(tag_address))
                .route_layer(middleware::from_fn_with_state(
                    STATE.clone(),
                    LongAlwaysCacheMiddleware::<false>::handler,
                )),
        )
        .nest(
            "/tags",
            Router::new()
                .route("/:tag", get(tag))
                .route_layer(middleware::from_fn_with_state(
                    STATE.clone(),
                    LongAlwaysCacheMiddleware::<true>::handler,
                )),
        )
        .with_state(STATE.clone())
}

pub async fn tag(
    Path(tag): Path<String>,
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;

    let results = postgres
        .query(
            "SELECT address, ARRAY_AGG(tag) AS tags
            FROM tags
            WHERE address IN (
                SELECT address
                FROM tags
                WHERE tag = $1
                ORDER BY id DESC
                OFFSET $2
                LIMIT $3
            )
            GROUP BY address",
            &[&tag, &pagination.offset(), &pagination.limit()],
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

pub async fn all_tags(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let postgres = state.postgres_pool.get().await?;

    let results = postgres
        .query("SELECT tag FROM tags GROUP BY tag", &[])
        .await?;

    let data = results
        .into_iter()
        .map(|row| Ok::<_, Error>(row.try_get::<_, String>("tag")?))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!({ "data": data })))
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
