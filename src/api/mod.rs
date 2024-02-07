use axum::{middleware, routing::get, Json, Router};
use serde_json::json;

use crate::{
    middleware::DefaultAlwaysCacheMiddleware,
    state::{State, STATE},
};

pub fn routes() -> Router<State> {
    Router::new()
        .route("/", get(|| async { Json(json!("Hello hehe")) }))
        .route_layer(middleware::from_fn_with_state(
            STATE.clone(),
            DefaultAlwaysCacheMiddleware::handler,
        ))
}
