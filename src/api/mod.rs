use axum::Router;

pub mod address;
pub mod block;
pub mod latest;
pub mod stats;
pub mod tag;
pub mod transaction;

pub fn routes() -> Router<()> {
    Router::new()
        .nest("/tx", transaction::routes())
        .nest("/block", block::routes())
        .nest("/address", address::routes())
        .nest("/latest", latest::routes())
        .nest("/tag", tag::routes())
        .nest("/stats", stats::routes())
}
