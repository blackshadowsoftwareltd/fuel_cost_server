use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::{
    auth::{hash_password, verify_password},
    database::{
        create_user, get_user_by_email, create_fuel_entry, create_fuel_entries, get_fuel_entries_by_user,
        get_fuel_entry_by_id, update_fuel_entry, delete_fuel_entry, delete_fuel_entries,
    },
    models::{SignupRequest, SigninRequest, AuthResponse, CreateFuelEntryRequest, CreateFuelEntriesRequest, UpdateFuelEntryRequest, DeleteFuelEntriesRequest},
};

pub async fn signup(
    State(pool): State<SqlitePool>,
    Json(request): Json<SignupRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if user already exists
    match get_user_by_email(&pool, &request.email).await {
        Ok(Some(_)) => {
            return Err((
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "User already exists",
                    "details": format!("A user with email '{}' already exists", request.email)
                }))
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
                }))
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
                }))
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
                }))
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
                    }))
                )),
                Err(e) => {
                    eprintln!("Password verification error: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Authentication failed",
                            "details": e.to_string()
                        }))
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
                        }))
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
                        }))
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
                }))
            ))
        }
    }
}

pub async fn create_fuel_entry_handler(
    State(pool): State<SqlitePool>,
    Json(request): Json<CreateFuelEntryRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // First validate that the user exists
    match sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&request.user_id)
        .fetch_optional(&pool)
        .await 
    {
        Ok(Some(_)) => {}, // User exists, continue
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid user ID",
                    "details": format!("No user found with id '{}'", request.user_id)
                }))
            ));
        }
        Err(e) => {
            eprintln!("Error validating user_id: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database validation error",
                    "details": e.to_string()
                }))
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
                    }))
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Failed to create fuel entry",
                        "details": error_msg
                    }))
                ))
            }
        }
    }
}

pub async fn create_fuel_entries_handler(
    State(pool): State<SqlitePool>,
    Json(request): Json<CreateFuelEntriesRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Validate that entries list is not empty
    if request.entries.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Empty entries list",
                "details": "At least one fuel entry must be provided"
            }))
        ));
    }

    // Validate that the user exists
    match sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&request.user_id)
        .fetch_optional(&pool)
        .await 
    {
        Ok(Some(_)) => {}, // User exists, continue
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid user ID",
                    "details": format!("No user found with id '{}'", request.user_id)
                }))
            ));
        }
        Err(e) => {
            eprintln!("Error validating user_id: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database validation error",
                    "details": e.to_string()
                }))
            ));
        }
    }

    match create_fuel_entries(&pool, &request.user_id, &request.entries).await {
        Ok(entries) => Ok(Json(json!({
            "message": format!("Successfully created {} fuel entries", entries.len()),
            "count": entries.len(),
            "entries": entries
        }))),
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
                    }))
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Failed to create fuel entries",
                        "details": error_msg
                    }))
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
                }))
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
            }))
        )),
        Err(e) => {
            eprintln!("Error getting fuel entry {} for user {}: {}", id, user_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get fuel entry",
                    "details": e.to_string()
                }))
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
            }))
        )),
        Err(e) => {
            eprintln!("Error updating fuel entry {} for user {}: {}", id, user_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to update fuel entry",
                    "details": e.to_string()
                }))
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
            }))
        )),
        Err(e) => {
            eprintln!("Error deleting fuel entry {} for user {}: {}", id, user_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to delete fuel entry",
                    "details": e.to_string()
                }))
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
            }))
        ));
    }

    // Validate that the user exists
    match sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&request.user_id)
        .fetch_optional(&pool)
        .await 
    {
        Ok(Some(_)) => {}, // User exists, continue
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid user ID",
                    "details": format!("No user found with id '{}'", request.user_id)
                }))
            ));
        }
        Err(e) => {
            eprintln!("Error validating user_id: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Database validation error",
                    "details": e.to_string()
                }))
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
                }))
            ))
        }
    }
}