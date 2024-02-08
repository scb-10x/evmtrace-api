use axum::{
    body::Body,
    extract::{OriginalUri, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use http_body_util::BodyExt;
use log::debug;
use redis::AsyncCommands;
use serde_json::{from_str, Value};

use crate::{error::AppError, state::State as AppState};

/// 10 seconds
pub const DEFAULT_CACHE_TTL: u32 = 10;
/// false
pub const DEFAULT_WITH_QUERY: bool = false;

pub type DefaultAlwaysCacheMiddleware =
    AlwaysCacheMiddleware<DEFAULT_CACHE_TTL, DEFAULT_WITH_QUERY>;
pub type AlwaysCacheWithQueryMiddleware<const CACHE_TTL: u32> =
    AlwaysCacheMiddleware<CACHE_TTL, true>;
pub type AlwaysCacheWithoutQueryMiddleware<const CACHE_TTL: u32> =
    AlwaysCacheMiddleware<CACHE_TTL, false>;

#[derive(Copy, Clone)]
pub struct AlwaysCacheMiddleware<
    const CACHE_TTL: u32 = DEFAULT_CACHE_TTL,
    const WITH_QUERY: bool = DEFAULT_WITH_QUERY,
>;

impl<const CACHE_TTL: u32, const WITH_QUERY: bool> AlwaysCacheMiddleware<CACHE_TTL, WITH_QUERY> {
    pub async fn handler(
        State(state): State<AppState>,
        OriginalUri(uri): OriginalUri,
        request: Request,
        next: Next,
    ) -> Result<Response, AppError> {
        let mut redis = state.redis_pool.aquire().await?;
        let key = format!(
            "{}:{}",
            match WITH_QUERY {
                true => uri.to_string(),
                false => uri.path().to_string(),
            },
            request.method()
        );
        debug!("Checking cache for key: {}", key);
        let cached_response = redis.get::<&str, Option<String>>(&key).await?;
        if let Some(cached_response) = cached_response {
            debug!("Cache hit!");
            return Ok(Json(from_str::<Value>(&cached_response)?).into_response());
        } else {
            debug!("Cache miss! Running handler...");
        }
        let response = next.run(request).await;
        let (parts, body) = response.into_parts();

        // check if error, if so, return response as is
        if parts.status.is_client_error() || parts.status.is_server_error() {
            return Ok(Response::from_parts(parts, body));
        }

        let bytes = body.collect().await?.to_bytes();
        let Json(body) = Json::<Value>::from_bytes(&bytes)?;
        redis
            .set_ex::<&str, String, ()>(&key, serde_json::to_string(&body)?, CACHE_TTL as u64)
            .await
            .ok();
        Ok(Response::from_parts(parts, Body::from(bytes)))
    }
}
