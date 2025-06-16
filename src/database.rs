use sqlx::SqlitePool;
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use std::fs;

use crate::models::{User, FuelEntry, FuelEntryDB, CreateFuelEntryRequest, FuelEntryData, UpdateFuelEntryRequest};

pub async fn create_database_pool() -> Result<SqlitePool> {
    // Create database file if it doesn't exist
    let db_path = "fuel_cost.db";
    if !std::path::Path::new(db_path).exists() {
        fs::File::create(db_path)?;
    }
    
    let pool = SqlitePool::connect("sqlite:fuel_cost.db").await?;
    Ok(pool)
}

pub async fn create_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fuel_entries (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            data TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users (id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_user(pool: &SqlitePool, email: &str, password_hash: &str) -> Result<User> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(email)
    .bind(password_hash)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(User {
        id,
        email: email.to_string(),
        password_hash: password_hash.to_string(),
        created_at,
    })
}

pub async fn get_user_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub async fn create_fuel_entry(
    pool: &SqlitePool,
    user_id: &str,
    request: &CreateFuelEntryRequest,
) -> Result<FuelEntry> {
    let id = Uuid::new_v4().to_string();
    
    let fuel_entry = FuelEntry {
        id: id.clone(),
        user_id: user_id.to_string(),
        liters: request.liters,
        price_per_liter: request.price_per_liter,
        total_cost: request.total_cost,
        date_time: request.date_time,
        odometer_reading: request.odometer_reading,
    };

    let data = serde_json::to_string(&fuel_entry)?;

    sqlx::query("INSERT INTO fuel_entries (id, user_id, data) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(user_id)
        .bind(&data)
        .execute(pool)
        .await?;

    Ok(fuel_entry)
}

pub async fn create_fuel_entries(
    pool: &SqlitePool,
    user_id: &str,
    entries_data: &[FuelEntryData],
) -> Result<Vec<FuelEntry>> {
    let mut created_entries = Vec::new();
    
    // Use a transaction to ensure all entries are created or none
    let mut tx = pool.begin().await?;
    
    for entry_data in entries_data {
        let id = Uuid::new_v4().to_string();
        
        let fuel_entry = FuelEntry {
            id: id.clone(),
            user_id: user_id.to_string(),
            liters: entry_data.liters,
            price_per_liter: entry_data.price_per_liter,
            total_cost: entry_data.total_cost,
            date_time: entry_data.date_time,
            odometer_reading: entry_data.odometer_reading,
        };

        let data = serde_json::to_string(&fuel_entry)?;

        sqlx::query("INSERT INTO fuel_entries (id, user_id, data) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(user_id)
            .bind(&data)
            .execute(&mut *tx)
            .await?;

        created_entries.push(fuel_entry);
    }
    
    // Commit the transaction
    tx.commit().await?;
    
    Ok(created_entries)
}

pub async fn get_fuel_entries_by_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<FuelEntry>> {
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    let mut entries = Vec::new();
    for entry_db in entries_db {
        let fuel_entry: FuelEntry = serde_json::from_str(&entry_db.data)?;
        entries.push(fuel_entry);
    }

    // Sort by date_time descending
    entries.sort_by(|a, b| b.date_time.cmp(&a.date_time));

    Ok(entries)
}

pub async fn get_fuel_entry_by_id(pool: &SqlitePool, id: &str, user_id: &str) -> Result<Option<FuelEntry>> {
    let entry_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    if let Some(entry_db) = entry_db {
        let fuel_entry: FuelEntry = serde_json::from_str(&entry_db.data)?;
        Ok(Some(fuel_entry))
    } else {
        Ok(None)
    }
}

pub async fn update_fuel_entry(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    request: &UpdateFuelEntryRequest,
) -> Result<Option<FuelEntry>> {
    let existing_entry = get_fuel_entry_by_id(pool, id, user_id).await?;
    
    if let Some(entry) = existing_entry {
        let updated_entry = FuelEntry {
            id: entry.id.clone(),
            user_id: entry.user_id,
            liters: request.liters.unwrap_or(entry.liters),
            price_per_liter: request.price_per_liter.unwrap_or(entry.price_per_liter),
            total_cost: request.total_cost.unwrap_or(entry.total_cost),
            date_time: request.date_time.unwrap_or(entry.date_time),
            odometer_reading: request.odometer_reading.or(entry.odometer_reading),
        };

        let data = serde_json::to_string(&updated_entry)?;

        sqlx::query("UPDATE fuel_entries SET data = ? WHERE id = ? AND user_id = ?")
            .bind(&data)
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(Some(updated_entry))
    } else {
        Ok(None)
    }
}

pub async fn delete_fuel_entry(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool> {
    // First check if the entry exists and belongs to the user
    let existing_entry = get_fuel_entry_by_id(pool, id, user_id).await?;
    
    if existing_entry.is_some() {
        let result = sqlx::query("DELETE FROM fuel_entries WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    } else {
        Ok(false)
    }
}

pub async fn delete_fuel_entries(
    pool: &SqlitePool,
    user_id: &str,
    entry_ids: &[String],
) -> Result<(usize, Vec<String>)> {
    if entry_ids.is_empty() {
        return Ok((0, Vec::new()));
    }

    let mut deleted_ids = Vec::new();
    let mut tx = pool.begin().await?;

    for entry_id in entry_ids {
        // Check if the entry exists and belongs to the user
        let existing_entry = sqlx::query_as::<_, FuelEntryDB>(
            "SELECT * FROM fuel_entries WHERE id = ? AND user_id = ?"
        )
        .bind(entry_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        if existing_entry.is_some() {
            // Delete the entry
            let result = sqlx::query("DELETE FROM fuel_entries WHERE id = ? AND user_id = ?")
                .bind(entry_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;

            if result.rows_affected() > 0 {
                deleted_ids.push(entry_id.clone());
            }
        }
    }

    // Commit the transaction
    tx.commit().await?;

    Ok((deleted_ids.len(), deleted_ids))
}