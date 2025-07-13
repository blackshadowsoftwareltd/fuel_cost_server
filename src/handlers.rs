use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, Json},
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::{
    auth::{hash_password, verify_password},
    database::{
        create_fuel_entries, create_fuel_entry, create_user, delete_fuel_entries,
        delete_fuel_entry, delete_user_by_id, get_all_users, get_dashboard_stats,
        get_fuel_entries_by_user, get_fuel_entry_by_id, get_service_status, get_user_by_email,
        is_service_enabled, update_fuel_entry, update_service_status,
    },
    models::{
        AdminActionRequest, AdminLoginRequest, AdminLoginResponse, AuthResponse,
        CreateFuelEntriesRequest, CreateFuelEntryRequest, DeleteFuelEntriesRequest,
        ServiceToggleRequest, SigninRequest, SignupRequest, UpdateFuelEntryRequest,
    },
};

pub async fn signup(
    State(pool): State<SqlitePool>,
    Json(request): Json<SignupRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if signin service is enabled
    match is_service_enabled(&pool, "signin").await {
        Ok(false) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "Service unavailable",
                    "details": "Sign-up service is currently disabled"
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error checking service status: {}", e);
        }
        _ => {}
    }

    // Check if user already exists
    match get_user_by_email(&pool, &request.email).await {
        Ok(Some(_)) => {
            return Err((
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "User already exists",
                    "details": format!("A user with email '{}' already exists", request.email)
                })),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("Database error during signup: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database error",
                    "details": e.to_string()
                })),
            ));
        }
    }

    // Hash password
    let password_hash = match hash_password(&request.password) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Password hashing error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Password processing failed",
                    "details": e.to_string()
                })),
            ));
        }
    };

    // Create user
    let user = match create_user(&pool, &request.email, &password_hash).await {
        Ok(user) => user,
        Err(e) => {
            eprintln!("User creation error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to create user",
                    "details": e.to_string()
                })),
            ));
        }
    };

    let response = AuthResponse {
        user_id: user.id,
        email: user.email,
    };

    Ok(Json(json!(response)))
}

pub async fn signin(
    State(pool): State<SqlitePool>,
    Json(request): Json<SigninRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if signin service is enabled
    match is_service_enabled(&pool, "signin").await {
        Ok(false) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "Service unavailable",
                    "details": "Sign-in service is currently disabled"
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error checking service status: {}", e);
        }
        _ => {}
    }

    // Find user by email
    match get_user_by_email(&pool, &request.email).await {
        Ok(Some(user)) => {
            // User exists, verify password
            match verify_password(&request.password, &user.password_hash) {
                Ok(true) => {
                    let response = AuthResponse {
                        user_id: user.id,
                        email: user.email,
                    };

                    Ok(Json(json!(response)))
                }
                Ok(false) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Invalid credentials",
                        "details": "Password is incorrect"
                    })),
                )),
                Err(e) => {
                    eprintln!("Password verification error: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Authentication failed",
                            "details": e.to_string()
                        })),
                    ))
                }
            }
        }
        Ok(None) => {
            // User doesn't exist, create new account
            let password_hash = match hash_password(&request.password) {
                Ok(hash) => hash,
                Err(e) => {
                    eprintln!("Password hashing error during auto-signup: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Password processing failed",
                            "details": e.to_string()
                        })),
                    ));
                }
            };

            let user = match create_user(&pool, &request.email, &password_hash).await {
                Ok(user) => user,
                Err(e) => {
                    eprintln!("Auto user creation error: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Failed to create user account",
                            "details": e.to_string()
                        })),
                    ));
                }
            };

            let response = AuthResponse {
                user_id: user.id,
                email: user.email,
            };

            Ok(Json(json!(response)))
        }
        Err(e) => {
            eprintln!("Database error during signin: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database error",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn create_fuel_entry_handler(
    State(pool): State<SqlitePool>,
    Json(request): Json<CreateFuelEntryRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if fuel entry service is enabled
    match is_service_enabled(&pool, "fuel_entry").await {
        Ok(false) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "Service unavailable",
                    "details": "Fuel entry service is currently disabled"
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error checking service status: {}", e);
        }
        _ => {}
    }

    // First validate that the user exists
    match sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&request.user_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(_)) => {} // User exists, continue
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid user ID",
                    "details": format!("No user found with id '{}'", request.user_id)
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error validating user_id: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database validation error",
                    "details": e.to_string()
                })),
            ));
        }
    }

    match create_fuel_entry(&pool, &request.user_id, &request).await {
        Ok(entry) => Ok(Json(json!(entry))),
        Err(e) => {
            eprintln!("Error creating fuel entry: {}", e);

            // Check for specific error types
            let error_msg = e.to_string();
            if error_msg.contains("FOREIGN KEY constraint failed") {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Invalid user ID",
                        "details": format!("User ID '{}' does not exist", request.user_id)
                    })),
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Failed to create fuel entry",
                        "details": error_msg
                    })),
                ))
            }
        }
    }
}

pub async fn create_fuel_entries_handler(
    State(pool): State<SqlitePool>,
    Json(request): Json<CreateFuelEntriesRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if fuel entry service is enabled
    match is_service_enabled(&pool, "fuel_entry").await {
        Ok(false) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "Service unavailable",
                    "details": "Fuel entry service is currently disabled"
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error checking service status: {}", e);
        }
        _ => {}
    }

    // Validate that entries list is not empty
    if request.entries.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Empty entries list",
                "details": "At least one fuel entry must be provided"
            })),
        ));
    }

    // Validate that the user exists
    match sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&request.user_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(_)) => {} // User exists, continue
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid user ID",
                    "details": format!("No user found with id '{}'", request.user_id)
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error validating user_id: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database validation error",
                    "details": e.to_string()
                })),
            ));
        }
    }

    let total_requested = request.entries.len();
    match create_fuel_entries(&pool, &request.user_id, &request.entries).await {
        Ok(entries) => {
            let created_count = entries.len();
            let duplicates_skipped = total_requested - created_count;

            Ok(Json(json!({
                "message": format!("Successfully processed {} entries: {} created, {} duplicates skipped",
                                  total_requested, created_count, duplicates_skipped),
                "total_requested": total_requested,
                "created_count": created_count,
                "duplicates_skipped": duplicates_skipped,
                "entries": entries
            })))
        }
        Err(e) => {
            eprintln!("Error creating fuel entries: {}", e);

            // Check for specific error types
            let error_msg = e.to_string();
            if error_msg.contains("FOREIGN KEY constraint failed") {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Invalid user ID",
                        "details": format!("User ID '{}' does not exist", request.user_id)
                    })),
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Failed to create fuel entries",
                        "details": error_msg
                    })),
                ))
            }
        }
    }
}

pub async fn get_fuel_entries_handler(
    State(pool): State<SqlitePool>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match get_fuel_entries_by_user(&pool, &user_id).await {
        Ok(entries) => Ok(Json(json!(entries))),
        Err(e) => {
            eprintln!("Error getting fuel entries for user {}: {}", user_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get fuel entries",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn get_fuel_entry_handler(
    State(pool): State<SqlitePool>,
    Path((user_id, id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match get_fuel_entry_by_id(&pool, &id, &user_id).await {
        Ok(Some(entry)) => Ok(Json(json!(entry))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Fuel entry not found",
                "details": format!("No fuel entry found with id '{}' for user '{}'", id, user_id)
            })),
        )),
        Err(e) => {
            eprintln!(
                "Error getting fuel entry {} for user {}: {}",
                id, user_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get fuel entry",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn update_fuel_entry_handler(
    State(pool): State<SqlitePool>,
    Path((user_id, id)): Path<(String, String)>,
    Json(request): Json<UpdateFuelEntryRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match update_fuel_entry(&pool, &id, &user_id, &request).await {
        Ok(Some(entry)) => Ok(Json(json!(entry))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Fuel entry not found",
                "details": format!("No fuel entry found with id '{}' for user '{}'", id, user_id)
            })),
        )),
        Err(e) => {
            eprintln!(
                "Error updating fuel entry {} for user {}: {}",
                id, user_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to update fuel entry",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn delete_fuel_entry_handler(
    State(pool): State<SqlitePool>,
    Path((user_id, id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match delete_fuel_entry(&pool, &id, &user_id).await {
        Ok(true) => Ok(Json(json!({"message": "Fuel entry deleted successfully"}))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Fuel entry not found",
                "details": format!("No fuel entry found with id '{}' for user '{}'", id, user_id)
            })),
        )),
        Err(e) => {
            eprintln!(
                "Error deleting fuel entry {} for user {}: {}",
                id, user_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to delete fuel entry",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn delete_fuel_entries_handler(
    State(pool): State<SqlitePool>,
    Json(request): Json<DeleteFuelEntriesRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Validate that entry_ids list is not empty
    if request.entry_ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Empty entry IDs list",
                "details": "At least one entry ID must be provided"
            })),
        ));
    }

    // Validate that the user exists
    match sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&request.user_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(_)) => {} // User exists, continue
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid user ID",
                    "details": format!("No user found with id '{}'", request.user_id)
                })),
            ));
        }
        Err(e) => {
            eprintln!("Error validating user_id: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database validation error",
                    "details": e.to_string()
                })),
            ));
        }
    }

    match delete_fuel_entries(&pool, &request.user_id, &request.entry_ids).await {
        Ok((deleted_count, deleted_ids)) => {
            let total_requested = request.entry_ids.len();
            let not_found = total_requested - deleted_count;

            Ok(Json(json!({
                "message": format!("Successfully deleted {} fuel entries", deleted_count),
                "deleted_count": deleted_count,
                "total_requested": total_requested,
                "not_found_count": not_found,
                "deleted_ids": deleted_ids
            })))
        }
        Err(e) => {
            eprintln!("Error deleting fuel entries: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to delete fuel entries",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn get_dashboard_handler(
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match get_dashboard_stats(&pool).await {
        Ok(stats) => Ok(Json(json!(stats))),
        Err(e) => {
            eprintln!("Error getting dashboard stats: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get dashboard statistics",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn get_all_users_handler(
    headers: HeaderMap,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check admin authentication
    if !verify_admin_token(&headers) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Unauthorized",
                "details": "Admin authentication required"
            })),
        ));
    }
    match get_all_users(&pool).await {
        Ok(users) => {
            let safe_users: Vec<_> = users
                .into_iter()
                .map(|user| {
                    json!({
                        "id": user.id,
                        "email": user.email,
                        "created_at": user.created_at
                    })
                })
                .collect();
            Ok(Json(json!(safe_users)))
        }
        Err(e) => {
            eprintln!("Error getting all users: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get users",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn admin_action_handler(
    headers: HeaderMap,
    State(pool): State<SqlitePool>,
    Json(request): Json<AdminActionRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check admin authentication
    if !verify_admin_token(&headers) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Unauthorized",
                "details": "Admin authentication required"
            })),
        ));
    }
    match request.action.as_str() {
        "delete_user" => {
            if let Some(user_id) = request.user_id {
                match delete_user_by_id(&pool, &user_id).await {
                    Ok(true) => Ok(Json(json!({
                        "message": "User deleted successfully",
                        "user_id": user_id
                    }))),
                    Ok(false) => Err((
                        StatusCode::NOT_FOUND,
                        Json(json!({
                            "error": "User not found",
                            "details": format!("No user found with id '{}'", user_id)
                        })),
                    )),
                    Err(e) => {
                        eprintln!("Error deleting user {}: {}", user_id, e);
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Failed to delete user",
                                "details": e.to_string()
                            })),
                        ))
                    }
                }
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Missing user_id",
                        "details": "user_id is required for delete_user action"
                    })),
                ))
            }
        }
        "delete_entry" => {
            if let (Some(user_id), Some(entry_id)) = (request.user_id, request.entry_id) {
                match delete_fuel_entry(&pool, &entry_id, &user_id).await {
                    Ok(true) => Ok(Json(json!({
                        "message": "Fuel entry deleted successfully",
                        "entry_id": entry_id,
                        "user_id": user_id
                    }))),
                    Ok(false) => Err((
                        StatusCode::NOT_FOUND,
                        Json(json!({
                            "error": "Fuel entry not found",
                            "details": format!("No fuel entry found with id '{}' for user '{}'", entry_id, user_id)
                        })),
                    )),
                    Err(e) => {
                        eprintln!(
                            "Error deleting fuel entry {} for user {}: {}",
                            entry_id, user_id, e
                        );
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Failed to delete fuel entry",
                                "details": e.to_string()
                            })),
                        ))
                    }
                }
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Missing parameters",
                        "details": "Both user_id and entry_id are required for delete_entry action"
                    })),
                ))
            }
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid action",
                "details": format!("Unknown action '{}'", request.action)
            })),
        )),
    }
}

pub async fn serve_dashboard() -> Html<&'static str> {
    Html(include_str!("../index.html"))
}

pub async fn admin_login_handler(
    Json(request): Json<AdminLoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Hardcoded admin credentials
    const ADMIN_EMAIL: &str = "me.remon.ahammad@bss.io";
    const ADMIN_PASSWORD: &str = "rustybustyrestapideshboard";

    if request.email == ADMIN_EMAIL && request.password == ADMIN_PASSWORD {
        // Generate a simple token (in production, use proper JWT)
        let token = format!("admin_token_{}", chrono::Utc::now().timestamp());

        Ok(Json(json!(AdminLoginResponse {
            success: true,
            token: Some(token),
            message: "Admin login successful".to_string(),
        })))
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!(AdminLoginResponse {
                success: false,
                token: None,
                message: "Invalid admin credentials".to_string(),
            })),
        ))
    }
}

pub async fn admin_verify_handler(
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer admin_token_") {
                return Ok(Json(json!({
                    "valid": true,
                    "message": "Token is valid"
                })));
            }
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "valid": false,
            "message": "Invalid or missing token"
        })),
    ))
}

// Helper function to verify admin token
fn verify_admin_token(headers: &HeaderMap) -> bool {
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            return auth_str.starts_with("Bearer admin_token_");
        }
    }
    false
}

pub async fn get_service_status_handler(
    headers: HeaderMap,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check admin authentication
    if !verify_admin_token(&headers) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Unauthorized",
                "details": "Admin authentication required"
            })),
        ));
    }
    match get_service_status(&pool).await {
        Ok(status) => Ok(Json(json!(status))),
        Err(e) => {
            eprintln!("Error getting service status: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get service status",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

pub async fn toggle_service_handler(
    headers: HeaderMap,
    State(pool): State<SqlitePool>,
    Json(request): Json<ServiceToggleRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check admin authentication
    if !verify_admin_token(&headers) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Unauthorized",
                "details": "Admin authentication required"
            })),
        ));
    }
    // Validate service name
    if !matches!(request.service.as_str(), "signin" | "fuel_entry") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid service",
                "details": format!("Unknown service '{}'", request.service)
            })),
        ));
    }

    match update_service_status(&pool, &request.service, request.enabled).await {
        Ok(true) => {
            let status_text = if request.enabled {
                "enabled"
            } else {
                "disabled"
            };
            Ok(Json(json!({
                "message": format!("Service '{}' has been {}", request.service, status_text),
                "service": request.service,
                "enabled": request.enabled
            })))
        }
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Service not found",
                "details": format!("Service '{}' not found", request.service)
            })),
        )),
        Err(e) => {
            eprintln!("Error toggling service {}: {}", request.service, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to toggle service",
                    "details": e.to_string()
                })),
            ))
        }
    }
}
