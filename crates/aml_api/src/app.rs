use crate::{auth::Keys, routes};
use aml_engine::Engine;
use axum::{extract::DefaultBodyLimit, middleware, Router};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    /// Rolling-window detection state. Held across "score + persist" so ingestion is serialised
    /// and the in-memory state can never run ahead of what the database has committed.
    pub engine: Arc<Mutex<Engine>>,
    pub keys: Arc<Keys>,
    pub hydrate_days: i64,
}

pub fn router(st: AppState) -> Router {
    Router::new()
        .merge(routes::router())
        .layer(middleware::from_fn_with_state(st.clone(), crate::auth::middleware))
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .layer(CorsLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(st)
}
