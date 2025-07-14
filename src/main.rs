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
    admin_action_handler, admin_login_handler, admin_verify_handler, create_fuel_entries_handler,
    create_fuel_entry_handler, delete_fuel_entries_handler, delete_fuel_entry_handler,
    get_all_users_handler, get_dashboard_handler, get_fuel_entries_handler, get_fuel_entry_handler,
    get_service_status_handler, serve_dashboard, signin, signup, toggle_service_handler,
    update_fuel_entry_handler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting fuel cost server...");
    println!("📝 Initializing tracing...");
    tracing_subscriber::fmt::init();
    println!("✅ Tracing initialized");

    // Create database pool and run migrations
    println!("🔌 Creating database pool...");
    let pool = create_database_pool().await?;
    println!("✅ Database pool created");
    // Create tables if they don't exist
    println!("📊 Creating tables...");
    create_tables(&pool).await?;
    println!("✅ Tables created");
    println!("🛣️ Building router...");
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
        // Admin authentication routes
        .route("/api/admin/login", post(admin_login_handler))
        .route("/api/admin/verify", get(admin_verify_handler))
        // Static files
        // .route("/", get(serve_dashboard))
        // .route("/dashboard", get(serve_dashboard))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(pool);
    println!("✅ Router built");

    // Run the server
    println!("🔗 Binding to 0.0.0.0:8880...");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8880").await?;
    println!("✅ Server bound to http://0.0.0.0:8880");

    println!("🎯 Starting server...");
    axum::serve(listener, app).await?;

    println!("❌ Server stopped unexpectedly"); // Should never reach here
    Ok(())
}
