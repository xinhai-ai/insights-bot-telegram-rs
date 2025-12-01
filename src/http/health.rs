use std::net::SocketAddr;

use axum::{Router, routing::get};
use tokio::task::JoinHandle;
use tracing::info;

use crate::db::Database;

async fn health_handler(db: Database) -> String {
    if sqlx::query("SELECT 1").execute(&db.pool).await.is_ok() {
        "ok".to_string()
    } else {
        "db_error".to_string()
    }
}

pub fn serve(db: Database, addr: SocketAddr) -> JoinHandle<()> {
    tokio::spawn(async move {
        let app = Router::new().route(
            "/health",
            get({
                let db = db.clone();
                move || health_handler(db.clone())
            }),
        );
        info!("starting health server on {addr}");
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.ok();
    })
}
