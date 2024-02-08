use axum::Router;

use crate::state::State;

pub mod transaction;

pub fn routes() -> Router<State> {
    Router::new().nest("/tx", transaction::routes())
}
