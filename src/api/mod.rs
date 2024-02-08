use axum::Router;

use crate::state::State;

pub mod address;
pub mod block;
pub mod transaction;

pub fn routes() -> Router<State> {
    Router::new()
        .nest("/tx", transaction::routes())
        .nest("/block", block::routes())
        .nest("/address", address::routes())
}
