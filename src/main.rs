mod auth;
mod database;
mod handlers;
mod models;

use axum::{
    response::Html,
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
    get_service_status_handler, signin, signup, toggle_service_handler, update_fuel_entry_handler,
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
        .route("/", get(root))
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

pub async fn root() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
  <title>Rust API Landing</title>
  <style>
    body {
      background-color: #1e1e1e;
      color: #ffffff;
      font-family: 'Comic Sans MS', cursive, sans-serif;
      text-align: center;
      padding-top: 5em;
      margin: 0;
    }

    h1 {
      font-size: 3em;
      margin-bottom: 0.2em;
    }

    p {
      font-size: 1.3em;
      margin-bottom: 2em;
    }

    .rust-logo {
      width: 100px;
      height: 100px;
      animation: spin 4s linear infinite;
      margin: 0 auto 2em;
      filter: brightness(0) invert(1); /* makes it white on dark bg */
    }

    @keyframes spin {
      0%   { transform: rotate(0deg); }
      100% { transform: rotate(360deg); }
    }

    .ascii {
      font-family: monospace;
      font-size: 0.9em;
      color: #ff7e00;
      margin-top: 2em;
    }

    .footer {
      margin-top: 4em;
      font-size: 0.8em;
      opacity: 0.6;
    }
    .ascii {
  font-family: monospace;
  font-size: 1em;
  color:rgb(255, 119, 0);
  margin-top: 2em;
  text-align: left;
  width: max-content;
  margin-left: auto;
  margin-right: auto;
  background-color:rgba(44, 44, 44, 0.59);
  padding: 0.5em;
  border-radius: 25px;
  box-shadow: 0 0 30px rgba(255, 0, 0, 0.32);
}

  </style>
</head>
<body>
  <h1>🦀 Welcome to the Rust API!</h1>
  <p>Zero-cost abstractions and 100% fun. Brace for <code>unsafe</code> levels of speed.</p>
  
  <img class="rust-logo" src="https://www.rust-lang.org/static/images/rust-logo-blk.svg" alt="Rust Logo" />

  <div class="ascii">
    <pre><code>
    fn main() {
        println!("Black Shadow Software");
    }
    </code></pre>
  </div>

  <div class="footer">
    This page was compiled in 0.0 seconds — probably. <br/>
    Try hitting <code>/api/v1/json</code> if you're looking for actual payloads.
  </div>
</body>
</html>
"#,
    )
}
