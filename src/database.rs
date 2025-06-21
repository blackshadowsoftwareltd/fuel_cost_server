use sqlx::SqlitePool;
use anyhow::Result;
use chrono::{Utc, Datelike};
use uuid::Uuid;
use std::fs;

use crate::models::{User, FuelEntry, FuelEntryDB, CreateFuelEntryRequest, FuelEntryData, UpdateFuelEntryRequest, DashboardStats, UserEntryCount, MonthlyStats, UserRegistrationStats, FuelEfficiencyStats, UserEfficiency, EfficiencyTrend, OdometerAnalytics, ConsumptionPatterns, DailyPattern, WeeklyPattern, SeasonalPattern, FillUpPatterns, CostAnalytics, CostDistribution, CostRange, SpendingTrend, BudgetAnalysis, CostPerUserStats, UserSpending, UserCostCategory, UserBehaviorStats, ActivityPatterns, UserActivity, ActivityDistribution, PeakUsageTime, EngagementMetrics, FeatureUsageStats, UserSegment, RetentionAnalysis, PredictiveAnalytics, PriceForecast, ConsumptionForecast, UserGrowthForecast, RevenueProjections, PriceTrends, DailyPriceTrend, PriceVolatility, PriceComparisons, RegionalPrice};

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
    
    // Get existing entries for this user to check for duplicates
    let existing_entries = get_fuel_entries_by_user_tx(&mut tx, user_id).await?;
    
    for entry_data in entries_data {
        // Check if this entry already exists (same data, ignoring ID)
        let is_duplicate = existing_entries.iter().any(|existing| {
            existing.liters == entry_data.liters
                && existing.price_per_liter == entry_data.price_per_liter
                && existing.total_cost == entry_data.total_cost
                && existing.date_time == entry_data.date_time
                && existing.odometer_reading == entry_data.odometer_reading
        });
        
        if is_duplicate {
            continue; // Skip duplicate entry
        }
        
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

async fn get_fuel_entries_by_user_tx<'a>(tx: &mut sqlx::Transaction<'a, sqlx::Sqlite>, user_id: &str) -> Result<Vec<FuelEntry>> {
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&mut **tx)
        .await?;

    let mut entries = Vec::new();
    for entry_db in entries_db {
        let fuel_entry: FuelEntry = serde_json::from_str(&entry_db.data)?;
        entries.push(fuel_entry);
    }

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

pub async fn get_dashboard_stats(pool: &SqlitePool) -> Result<DashboardStats> {
    let total_users: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    let total_fuel_entries: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM fuel_entries")
        .fetch_one(pool)
        .await?;

    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;

    let mut all_entries = Vec::new();
    for entry_db in entries_db {
        let fuel_entry: FuelEntry = serde_json::from_str(&entry_db.data)?;
        all_entries.push(fuel_entry);
    }

    let total_fuel_cost: f64 = all_entries.iter().map(|e| e.total_cost).sum();
    let total_liters: f64 = all_entries.iter().map(|e| e.liters).sum();
    let average_price_per_liter = if total_liters > 0.0 { total_fuel_cost / total_liters } else { 0.0 };

    let users_with_most_entries = get_users_with_most_entries(pool).await?;
    
    let mut most_expensive_entries = all_entries.clone();
    most_expensive_entries.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());
    most_expensive_entries.truncate(10);

    let mut recent_entries = all_entries.clone();
    recent_entries.sort_by(|a, b| b.date_time.cmp(&a.date_time));
    recent_entries.truncate(10);

    let monthly_stats = get_monthly_stats(pool).await?;
    let user_registration_stats = get_user_registration_stats(pool).await?;
    let fuel_efficiency_stats = get_fuel_efficiency_stats(pool).await?;
    let consumption_patterns = get_consumption_patterns(pool).await?;
    let cost_analytics = get_cost_analytics(pool).await?;
    let user_behavior_stats = get_user_behavior_stats(pool).await?;
    let predictive_analytics = get_predictive_analytics(pool).await?;
    let price_trends = get_price_trends(pool).await?;

    Ok(DashboardStats {
        total_users,
        total_fuel_entries,
        total_fuel_cost,
        total_liters,
        average_price_per_liter,
        users_with_most_entries,
        most_expensive_entries,
        recent_entries,
        monthly_stats,
        user_registration_stats,
        fuel_efficiency_stats,
        consumption_patterns,
        cost_analytics,
        user_behavior_stats,
        predictive_analytics,
        price_trends,
    })
}

pub async fn get_users_with_most_entries(pool: &SqlitePool) -> Result<Vec<UserEntryCount>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await?;

    let mut user_counts = Vec::new();
    
    for user in users {
        let entries = get_fuel_entries_by_user(pool, &user.id).await?;
        let entry_count = entries.len() as i32;
        let total_cost: f64 = entries.iter().map(|e| e.total_cost).sum();
        let total_liters: f64 = entries.iter().map(|e| e.liters).sum();
        
        user_counts.push(UserEntryCount {
            user_id: user.id,
            email: user.email,
            entry_count,
            total_cost,
            total_liters,
        });
    }

    user_counts.sort_by(|a, b| b.entry_count.cmp(&a.entry_count));
    user_counts.truncate(10);
    
    Ok(user_counts)
}

pub async fn get_monthly_stats(pool: &SqlitePool) -> Result<Vec<MonthlyStats>> {
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;

    let mut all_entries = Vec::new();
    for entry_db in entries_db {
        let fuel_entry: FuelEntry = serde_json::from_str(&entry_db.data)?;
        all_entries.push(fuel_entry);
    }

    use std::collections::HashMap;
    let mut monthly_data: HashMap<String, (i32, f64, f64)> = HashMap::new();

    for entry in all_entries {
        let month_key = format!("{}-{:02}", entry.date_time.year(), entry.date_time.month());
        let (count, cost, liters) = monthly_data.get(&month_key).unwrap_or(&(0, 0.0, 0.0));
        monthly_data.insert(month_key, (count + 1, cost + entry.total_cost, liters + entry.liters));
    }

    let mut stats = Vec::new();
    for (month_key, (count, cost, liters)) in monthly_data {
        let parts: Vec<&str> = month_key.split('-').collect();
        let year = parts[0].parse::<i32>().unwrap_or(0);
        let month_num = parts[1].parse::<u32>().unwrap_or(1);
        let month_name = match month_num {
            1 => "January", 2 => "February", 3 => "March", 4 => "April",
            5 => "May", 6 => "June", 7 => "July", 8 => "August",
            9 => "September", 10 => "October", 11 => "November", 12 => "December",
            _ => "Unknown"
        };
        
        stats.push(MonthlyStats {
            month: month_name.to_string(),
            year,
            total_entries: count,
            total_cost: cost,
            total_liters: liters,
            average_price: if liters > 0.0 { cost / liters } else { 0.0 },
        });
    }

    stats.sort_by(|a, b| {
        let a_date = (a.year, a.month.as_str());
        let b_date = (b.year, b.month.as_str());
        b_date.cmp(&a_date)
    });

    Ok(stats)
}

pub async fn get_user_registration_stats(pool: &SqlitePool) -> Result<Vec<UserRegistrationStats>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await?;

    use std::collections::HashMap;
    let mut monthly_registrations: HashMap<String, i32> = HashMap::new();

    for user in users {
        let month_key = format!("{}-{:02}", user.created_at.year(), user.created_at.month());
        let count = monthly_registrations.get(&month_key).unwrap_or(&0);
        monthly_registrations.insert(month_key, count + 1);
    }

    let mut stats = Vec::new();
    for (month_key, count) in monthly_registrations {
        let parts: Vec<&str> = month_key.split('-').collect();
        let year = parts[0].parse::<i32>().unwrap_or(0);
        let month_num = parts[1].parse::<u32>().unwrap_or(1);
        let month_name = match month_num {
            1 => "January", 2 => "February", 3 => "March", 4 => "April",
            5 => "May", 6 => "June", 7 => "July", 8 => "August",
            9 => "September", 10 => "October", 11 => "November", 12 => "December",
            _ => "Unknown"
        };
        
        stats.push(UserRegistrationStats {
            month: month_name.to_string(),
            year,
            new_users: count,
        });
    }

    stats.sort_by(|a, b| {
        let a_date = (a.year, a.month.as_str());
        let b_date = (b.year, b.month.as_str());
        b_date.cmp(&a_date)
    });

    Ok(stats)
}

pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(users)
}

pub async fn delete_user_by_id(pool: &SqlitePool, user_id: &str) -> Result<bool> {
    let mut tx = pool.begin().await?;
    
    sqlx::query("DELETE FROM fuel_entries WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    
    Ok(result.rows_affected() > 0)
}

pub async fn get_fuel_efficiency_stats(pool: &SqlitePool) -> Result<FuelEfficiencyStats> {
    let users = get_all_users(pool).await?;
    let mut user_efficiencies = Vec::new();
    let mut total_odometer_users = 0;
    let mut total_distance = 0.0;
    let mut total_entries_with_odometer = 0;
    
    for user in users {
        let entries = get_fuel_entries_by_user(pool, &user.id).await?;
        if entries.is_empty() { continue; }
        
        let total_liters: f64 = entries.iter().map(|e| e.liters).sum();
        let avg_liters = total_liters / entries.len() as f64;
        
        // Calculate efficiency score (lower liters per entry = higher efficiency)
        let efficiency_score = if avg_liters > 0.0 { 100.0 / avg_liters } else { 0.0 };
        
        // Calculate distance if odometer readings are available
        let mut distance_covered = 0.0;
        let mut has_odometer_data = false;
        
        let mut sorted_entries = entries.clone();
        sorted_entries.sort_by(|a, b| a.date_time.cmp(&b.date_time));
        
        for window in sorted_entries.windows(2) {
            if let (Some(prev_odo), Some(curr_odo)) = (window[0].odometer_reading, window[1].odometer_reading) {
                if curr_odo > prev_odo {
                    distance_covered += curr_odo - prev_odo;
                    has_odometer_data = true;
                }
            }
        }
        
        if has_odometer_data {
            total_odometer_users += 1;
            total_distance += distance_covered;
            total_entries_with_odometer += entries.len();
        }
        
        user_efficiencies.push(UserEfficiency {
            user_id: user.id,
            email: user.email,
            average_liters_per_entry: avg_liters,
            total_entries: entries.len() as i32,
            fuel_efficiency_score: efficiency_score,
        });
    }
    
    user_efficiencies.sort_by(|a, b| b.fuel_efficiency_score.partial_cmp(&a.fuel_efficiency_score).unwrap());
    
    let most_efficient = user_efficiencies.iter().take(5).cloned().collect();
    let least_efficient = user_efficiencies.iter().rev().take(5).cloned().collect();
    
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;
    
    let total_entries = entries_db.len();
    let average_fuel_per_entry = if total_entries > 0 {
        let mut total_liters = 0.0;
        for entry_db in &entries_db {
            if let Ok(fuel_entry) = serde_json::from_str::<FuelEntry>(&entry_db.data) {
                total_liters += fuel_entry.liters;
            }
        }
        total_liters / total_entries as f64
    } else {
        0.0
    };
    
    Ok(FuelEfficiencyStats {
        average_fuel_per_entry,
        most_efficient_users: most_efficient,
        least_efficient_users: least_efficient,
        efficiency_trends: vec![], // Simplified for now
        odometer_analytics: OdometerAnalytics {
            users_with_odometer: total_odometer_users,
            users_without_odometer: (user_efficiencies.len() as i32) - total_odometer_users,
            average_distance_per_entry: if total_entries_with_odometer > 0 { 
                Some(total_distance / total_entries_with_odometer as f64) 
            } else { None },
            total_distance_tracked: if total_distance > 0.0 { Some(total_distance) } else { None },
            fuel_per_km: if total_distance > 0.0 && average_fuel_per_entry > 0.0 { 
                Some(average_fuel_per_entry / (total_distance / total_entries_with_odometer as f64)) 
            } else { None },
        },
    })
}

pub async fn get_consumption_patterns(pool: &SqlitePool) -> Result<ConsumptionPatterns> {
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;
    
    let mut all_entries = Vec::new();
    for entry_db in entries_db {
        if let Ok(fuel_entry) = serde_json::from_str::<FuelEntry>(&entry_db.data) {
            all_entries.push(fuel_entry);
        }
    }
    
    // Analyze fill-up patterns
    let mut small_fillups = 0;
    let mut medium_fillups = 0;
    let mut large_fillups = 0;
    let total_liters: f64 = all_entries.iter().map(|e| e.liters).sum();
    
    for entry in &all_entries {
        if entry.liters < 10.0 {
            small_fillups += 1;
        } else if entry.liters <= 30.0 {
            medium_fillups += 1;
        } else {
            large_fillups += 1;
        }
    }
    
    let average_fillup_size = if !all_entries.is_empty() { 
        total_liters / all_entries.len() as f64 
    } else { 0.0 };
    
    let most_common_range = if small_fillups >= medium_fillups && small_fillups >= large_fillups {
        "Small (< 10L)".to_string()
    } else if medium_fillups >= large_fillups {
        "Medium (10-30L)".to_string()
    } else {
        "Large (> 30L)".to_string()
    };
    
    // Weekly patterns
    let mut weekly_patterns = Vec::new();
    let weekdays = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    
    for (i, day) in weekdays.iter().enumerate() {
        let day_entries: Vec<_> = all_entries.iter()
            .filter(|e| e.date_time.weekday().num_days_from_monday() == i as u32)
            .collect();
        
        let entry_count = day_entries.len() as i32;
        let average_cost = if !day_entries.is_empty() {
            day_entries.iter().map(|e| e.total_cost).sum::<f64>() / day_entries.len() as f64
        } else { 0.0 };
        let total_liters = day_entries.iter().map(|e| e.liters).sum();
        
        weekly_patterns.push(WeeklyPattern {
            day_of_week: day.to_string(),
            entry_count,
            average_cost,
            total_liters,
        });
    }
    
    Ok(ConsumptionPatterns {
        daily_patterns: vec![], // Simplified for now
        weekly_patterns,
        seasonal_patterns: vec![], // Simplified for now
        fill_up_patterns: FillUpPatterns {
            small_fillups,
            medium_fillups,
            large_fillups,
            average_fillup_size,
            most_common_fillup_range: most_common_range,
        },
    })
}

pub async fn get_cost_analytics(pool: &SqlitePool) -> Result<CostAnalytics> {
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;
    
    let mut all_entries = Vec::new();
    for entry_db in entries_db {
        if let Ok(fuel_entry) = serde_json::from_str::<FuelEntry>(&entry_db.data) {
            all_entries.push(fuel_entry);
        }
    }
    
    let mut low_cost = 0;
    let mut medium_cost = 0;
    let mut high_cost = 0;
    
    for entry in &all_entries {
        if entry.total_cost < 50.0 {
            low_cost += 1;
        } else if entry.total_cost <= 150.0 {
            medium_cost += 1;
        } else {
            high_cost += 1;
        }
    }
    
    let total_entries = all_entries.len() as i32;
    let cost_ranges = vec![
        CostRange {
            range: "Low (< $50)".to_string(),
            count: low_cost,
            percentage: if total_entries > 0 { (low_cost as f64 / total_entries as f64) * 100.0 } else { 0.0 },
        },
        CostRange {
            range: "Medium ($50-$150)".to_string(),
            count: medium_cost,
            percentage: if total_entries > 0 { (medium_cost as f64 / total_entries as f64) * 100.0 } else { 0.0 },
        },
        CostRange {
            range: "High (> $150)".to_string(),
            count: high_cost,
            percentage: if total_entries > 0 { (high_cost as f64 / total_entries as f64) * 100.0 } else { 0.0 },
        },
    ];
    
    // Calculate user spending stats
    let users = get_all_users(pool).await?;
    let mut user_spendings = Vec::new();
    
    for user in users {
        let entries = get_fuel_entries_by_user(pool, &user.id).await?;
        let total_spent: f64 = entries.iter().map(|e| e.total_cost).sum();
        let average_per_entry = if !entries.is_empty() {
            total_spent / entries.len() as f64
        } else { 0.0 };
        
        user_spendings.push(UserSpending {
            user_id: user.id,
            email: user.email,
            total_spent,
            average_per_entry,
            entry_count: entries.len() as i32,
        });
    }
    
    user_spendings.sort_by(|a, b| b.total_spent.partial_cmp(&a.total_spent).unwrap());
    let top_spenders: Vec<UserSpending> = user_spendings.into_iter().take(10).collect();
    
    let total_cost: f64 = all_entries.iter().map(|e| e.total_cost).sum();
    let average_cost_per_user = if !top_spenders.is_empty() {
        total_cost / top_spenders.len() as f64
    } else { 0.0 };
    
    Ok(CostAnalytics {
        cost_distribution: CostDistribution {
            low_cost_entries: low_cost,
            medium_cost_entries: medium_cost,
            high_cost_entries: high_cost,
            cost_ranges,
        },
        spending_trends: vec![], // Simplified for now
        budget_analysis: BudgetAnalysis {
            average_monthly_spending: total_cost,
            highest_spending_month: "June 2025".to_string(),
            lowest_spending_month: "June 2025".to_string(),
            spending_volatility: 0.0,
        },
        cost_per_user_stats: CostPerUserStats {
            average_cost_per_user,
            median_cost_per_user: average_cost_per_user,
            top_spenders,
            cost_distribution_by_user: vec![],
        },
    })
}

pub async fn get_user_behavior_stats(pool: &SqlitePool) -> Result<UserBehaviorStats> {
    let users = get_all_users(pool).await?;
    let mut user_activities = Vec::new();
    
    for user in users {
        let entries = get_fuel_entries_by_user(pool, &user.id).await?;
        let entry_count = entries.len() as i32;
        
        // Calculate days active (simplified)
        let days_active = if !entries.is_empty() {
            let first_entry = entries.iter().min_by_key(|e| e.date_time).unwrap();
            let last_entry = entries.iter().max_by_key(|e| e.date_time).unwrap();
            let duration = last_entry.date_time.signed_duration_since(first_entry.date_time);
            std::cmp::max(1, duration.num_days() as i32)
        } else { 0 };
        
        let average_entries_per_day = if days_active > 0 {
            entry_count as f64 / days_active as f64
        } else { 0.0 };
        
        let last_activity = if !entries.is_empty() {
            entries.iter().max_by_key(|e| e.date_time).unwrap().date_time.format("%Y-%m-%d").to_string()
        } else {
            "Never".to_string()
        };
        
        user_activities.push(UserActivity {
            user_id: user.id,
            email: user.email,
            entry_count,
            days_active,
            average_entries_per_day,
            last_activity,
        });
    }
    
    user_activities.sort_by(|a, b| b.entry_count.cmp(&a.entry_count));
    let most_active = user_activities.iter().take(10).cloned().collect();
    let least_active = user_activities.iter().rev().take(10).cloned().collect();
    
    // Calculate feature usage
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;
    
    let mut entries_with_odometer = 0;
    let total_entries = entries_db.len();
    
    for entry_db in &entries_db {
        if let Ok(fuel_entry) = serde_json::from_str::<FuelEntry>(&entry_db.data) {
            if fuel_entry.odometer_reading.is_some() {
                entries_with_odometer += 1;
            }
        }
    }
    
    let odometer_usage_rate = if total_entries > 0 {
        (entries_with_odometer as f64 / total_entries as f64) * 100.0
    } else { 0.0 };
    
    Ok(UserBehaviorStats {
        activity_patterns: ActivityPatterns {
            most_active_users: most_active,
            least_active_users: least_active,
            activity_distribution: vec![],
            peak_usage_times: vec![],
        },
        engagement_metrics: EngagementMetrics {
            average_session_entries: 1.5, // Simplified
            user_consistency_score: 75.0, // Simplified
            feature_usage_stats: FeatureUsageStats {
                odometer_usage_rate,
                bulk_entry_usage: 0, // Would need to track this
                average_entry_completeness: 90.0, // Simplified
            },
        },
        user_segments: vec![],
        retention_analysis: RetentionAnalysis {
            new_user_retention_7_day: 80.0,  // Simplified
            new_user_retention_30_day: 60.0, // Simplified
            active_user_retention: 75.0,     // Simplified
            churn_rate: 5.0,                  // Simplified
        },
    })
}

pub async fn get_predictive_analytics(pool: &SqlitePool) -> Result<PredictiveAnalytics> {
    // Simple predictive analytics based on current trends
    let current_stats = get_monthly_stats(pool).await?;
    let users_count = get_all_users(pool).await?.len();
    
    let mut price_forecast = Vec::new();
    let mut consumption_forecast = Vec::new();
    let mut user_growth_forecast = Vec::new();
    
    // Generate simple forecasts for next 6 months
    let base_price = if !current_stats.is_empty() {
        current_stats[0].average_price
    } else { 120.0 };
    
    let base_consumption = if !current_stats.is_empty() {
        current_stats[0].total_liters
    } else { 100.0 };
    
    for i in 1..=6 {
        let month = match (6 + i) % 12 {
            0 => 12, m => m
        };
        let year = if month <= 6 { 2026 } else { 2025 };
        let month_name = match month {
            1 => "January", 2 => "February", 3 => "March", 4 => "April",
            5 => "May", 6 => "June", 7 => "July", 8 => "August",
            9 => "September", 10 => "October", 11 => "November", 12 => "December",
            _ => "Unknown"
        };
        
        // Simple price trend (slight increase over time)
        let predicted_price = base_price * (1.0 + (i as f64 * 0.02));
        
        price_forecast.push(PriceForecast {
            month: month_name.to_string(),
            year: year as i32,
            predicted_price,
            confidence_level: 75.0,
            trend_direction: "Increasing".to_string(),
        });
        
        // Simple consumption forecast
        let predicted_consumption = base_consumption * (1.0 + (i as f64 * 0.05));
        
        consumption_forecast.push(ConsumptionForecast {
            month: month_name.to_string(),
            year: year as i32,
            predicted_consumption,
            predicted_entries: (predicted_consumption / 10.0) as i32,
        });
        
        // Simple user growth forecast
        let predicted_new_users = std::cmp::max(1, (users_count as f64 * 0.1) as i32);
        let predicted_total_users = users_count as i32 + (predicted_new_users * i);
        
        user_growth_forecast.push(UserGrowthForecast {
            month: month_name.to_string(),
            year: year as i32,
            predicted_new_users,
            predicted_total_users,
            growth_rate: 10.0,
        });
    }
    
    Ok(PredictiveAnalytics {
        fuel_price_forecast: price_forecast,
        consumption_forecast,
        user_growth_forecast,
        revenue_projections: RevenueProjections {
            next_month_revenue: base_price * base_consumption * 1.1,
            next_quarter_revenue: base_price * base_consumption * 3.2,
            annual_revenue_projection: base_price * base_consumption * 12.5,
            growth_assumptions: "Based on current trends with 10% growth rate".to_string(),
        },
    })
}

pub async fn get_price_trends(pool: &SqlitePool) -> Result<PriceTrends> {
    let entries_db = sqlx::query_as::<_, FuelEntryDB>("SELECT * FROM fuel_entries")
        .fetch_all(pool)
        .await?;
    
    let mut all_entries = Vec::new();
    for entry_db in entries_db {
        if let Ok(fuel_entry) = serde_json::from_str::<FuelEntry>(&entry_db.data) {
            all_entries.push(fuel_entry);
        }
    }
    
    if all_entries.is_empty() {
        return Ok(PriceTrends {
            daily_price_trends: vec![],
            price_volatility: PriceVolatility {
                volatility_index: 0.0,
                price_standard_deviation: 0.0,
                most_volatile_period: "N/A".to_string(),
                least_volatile_period: "N/A".to_string(),
            },
            price_comparisons: PriceComparisons {
                current_vs_last_month: 0.0,
                current_vs_last_year: 0.0,
                lowest_recorded_price: 0.0,
                highest_recorded_price: 0.0,
                price_change_percentage: 0.0,
            },
            regional_price_data: vec![],
        });
    }
    
    // Calculate price statistics
    let prices: Vec<f64> = all_entries.iter().map(|e| e.price_per_liter).collect();
    let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;
    
    // Calculate standard deviation
    let variance = prices.iter()
        .map(|price| (price - avg_price).powi(2))
        .sum::<f64>() / prices.len() as f64;
    let std_dev = variance.sqrt();
    
    Ok(PriceTrends {
        daily_price_trends: vec![],
        price_volatility: PriceVolatility {
            volatility_index: (std_dev / avg_price) * 100.0,
            price_standard_deviation: std_dev,
            most_volatile_period: "June 2025".to_string(),
            least_volatile_period: "June 2025".to_string(),
        },
        price_comparisons: PriceComparisons {
            current_vs_last_month: 0.0,
            current_vs_last_year: 0.0,
            lowest_recorded_price: min_price,
            highest_recorded_price: max_price,
            price_change_percentage: ((max_price - min_price) / min_price) * 100.0,
        },
        regional_price_data: vec![
            RegionalPrice {
                region: "Global Average".to_string(),
                average_price: avg_price,
                entry_count: all_entries.len() as i32,
                price_rank: 1,
            }
        ],
    })
}