use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FuelEntryDB {
    pub id: String,
    pub user_id: String,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FuelEntry {
    pub id: String,
    pub user_id: String,
    pub liters: f64,
    pub price_per_liter: f64,
    pub total_cost: f64,
    pub date_time: DateTime<Utc>,
    pub odometer_reading: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SigninRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFuelEntryRequest {
    pub user_id: String,
    pub liters: f64,
    pub price_per_liter: f64,
    pub total_cost: f64,
    pub date_time: DateTime<Utc>,
    pub odometer_reading: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFuelEntriesRequest {
    pub user_id: String,
    pub entries: Vec<FuelEntryData>,
}

#[derive(Debug, Deserialize)]
pub struct FuelEntryData {
    pub liters: f64,
    pub price_per_liter: f64,
    pub total_cost: f64,
    pub date_time: DateTime<Utc>,
    pub odometer_reading: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFuelEntryRequest {
    pub liters: Option<f64>,
    pub price_per_liter: Option<f64>,
    pub total_cost: Option<f64>,
    pub date_time: Option<DateTime<Utc>>,
    pub odometer_reading: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFuelEntriesRequest {
    pub user_id: String,
    pub entry_ids: Vec<String>,
}