mod auth;
mod database;
mod handlers;
mod models;

use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber;

use database::{create_database_pool, create_tables};
use handlers::{
    create_fuel_entries_handler, create_fuel_entry_handler, delete_fuel_entries_handler,
    delete_fuel_entry_handler, get_fuel_entries_handler, get_fuel_entry_handler, signin, signup,
    update_fuel_entry_handler, get_dashboard_handler, get_all_users_handler, admin_action_handler,
    serve_dashboard, get_service_status_handler, toggle_service_handler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create database pool and run migrations
    let pool = create_database_pool().await?;

    // Create tables if they don't exist
    create_tables(&pool).await?;

    // Build our application with routes
    let app = Router::new()
        // Auth routes
        .route("/api/auth/signup", post(signup))
        .route("/api/auth/signin", post(signin))
        // Fuel entry routes
        .route("/api/fuel-entries", post(create_fuel_entry_handler))
        .route("/api/fuel-entries/bulk", post(create_fuel_entries_handler))
        .route(
            "/api/fuel-entries/bulk/delete",
            post(delete_fuel_entries_handler),
        )
        .route("/api/fuel-entries/:user_id", get(get_fuel_entries_handler))
        .route(
            "/api/fuel-entries/:user_id/:id",
            get(get_fuel_entry_handler)
                .put(update_fuel_entry_handler)
                .delete(delete_fuel_entry_handler),
        )
        // Dashboard routes
        .route("/api/dashboard", get(get_dashboard_handler))
        .route("/api/admin/users", get(get_all_users_handler))
        .route("/api/admin/action", post(admin_action_handler))
        .route("/api/admin/service-status", get(get_service_status_handler))
        .route("/api/admin/service-toggle", post(toggle_service_handler))
        // Static files
        .route("/", get(serve_dashboard))
        .route("/dashboard", get(serve_dashboard))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(pool);

    // Run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8880").await?;
    println!("Server running on http://127.0.0.1:8880");

    axum::serve(listener, app).await?;

    Ok(())
}
