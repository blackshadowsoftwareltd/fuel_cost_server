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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_users: i32,
    pub total_fuel_entries: i32,
    pub total_fuel_cost: f64,
    pub total_liters: f64,
    pub average_price_per_liter: f64,
    pub users_with_most_entries: Vec<UserEntryCount>,
    pub most_expensive_entries: Vec<FuelEntry>,
    pub recent_entries: Vec<FuelEntry>,
    pub monthly_stats: Vec<MonthlyStats>,
    pub user_registration_stats: Vec<UserRegistrationStats>,
    pub fuel_efficiency_stats: FuelEfficiencyStats,
    pub consumption_patterns: ConsumptionPatterns,
    pub cost_analytics: CostAnalytics,
    pub user_behavior_stats: UserBehaviorStats,
    pub predictive_analytics: PredictiveAnalytics,
    pub price_trends: PriceTrends,
}

#[derive(Debug, Serialize)]
pub struct UserEntryCount {
    pub user_id: String,
    pub email: String,
    pub entry_count: i32,
    pub total_cost: f64,
    pub total_liters: f64,
}

#[derive(Debug, Serialize)]
pub struct MonthlyStats {
    pub month: String,
    pub year: i32,
    pub total_entries: i32,
    pub total_cost: f64,
    pub total_liters: f64,
    pub average_price: f64,
}

#[derive(Debug, Serialize)]
pub struct UserRegistrationStats {
    pub month: String,
    pub year: i32,
    pub new_users: i32,
}

#[derive(Debug, Deserialize)]
pub struct AdminActionRequest {
    pub action: String,
    pub user_id: Option<String>,
    pub entry_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FuelEfficiencyStats {
    pub average_fuel_per_entry: f64,
    pub most_efficient_users: Vec<UserEfficiency>,
    pub least_efficient_users: Vec<UserEfficiency>,
    pub efficiency_trends: Vec<EfficiencyTrend>,
    pub odometer_analytics: OdometerAnalytics,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserEfficiency {
    pub user_id: String,
    pub email: String,
    pub average_liters_per_entry: f64,
    pub total_entries: i32,
    pub fuel_efficiency_score: f64,
}

#[derive(Debug, Serialize)]
pub struct EfficiencyTrend {
    pub month: String,
    pub year: i32,
    pub average_efficiency: f64,
    pub total_distance: Option<f64>,
    pub fuel_consumption_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct OdometerAnalytics {
    pub users_with_odometer: i32,
    pub users_without_odometer: i32,
    pub average_distance_per_entry: Option<f64>,
    pub total_distance_tracked: Option<f64>,
    pub fuel_per_km: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ConsumptionPatterns {
    pub daily_patterns: Vec<DailyPattern>,
    pub weekly_patterns: Vec<WeeklyPattern>,
    pub seasonal_patterns: Vec<SeasonalPattern>,
    pub fill_up_patterns: FillUpPatterns,
}

#[derive(Debug, Serialize)]
pub struct DailyPattern {
    pub hour_of_day: i32,
    pub entry_count: i32,
    pub average_cost: f64,
    pub average_liters: f64,
}

#[derive(Debug, Serialize)]
pub struct WeeklyPattern {
    pub day_of_week: String,
    pub entry_count: i32,
    pub average_cost: f64,
    pub total_liters: f64,
}

#[derive(Debug, Serialize)]
pub struct SeasonalPattern {
    pub season: String,
    pub entry_count: i32,
    pub average_price_per_liter: f64,
    pub total_cost: f64,
}

#[derive(Debug, Serialize)]
pub struct FillUpPatterns {
    pub small_fillups: i32,    // < 10 liters
    pub medium_fillups: i32,   // 10-30 liters
    pub large_fillups: i32,    // > 30 liters
    pub average_fillup_size: f64,
    pub most_common_fillup_range: String,
}

#[derive(Debug, Serialize)]
pub struct CostAnalytics {
    pub cost_distribution: CostDistribution,
    pub spending_trends: Vec<SpendingTrend>,
    pub budget_analysis: BudgetAnalysis,
    pub cost_per_user_stats: CostPerUserStats,
}

#[derive(Debug, Serialize)]
pub struct CostDistribution {
    pub low_cost_entries: i32,     // < $50
    pub medium_cost_entries: i32,  // $50-$150
    pub high_cost_entries: i32,    // > $150
    pub cost_ranges: Vec<CostRange>,
}

#[derive(Debug, Serialize)]
pub struct CostRange {
    pub range: String,
    pub count: i32,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct SpendingTrend {
    pub month: String,
    pub year: i32,
    pub total_spending: f64,
    pub average_per_entry: f64,
    pub spending_growth: f64,
}

#[derive(Debug, Serialize)]
pub struct BudgetAnalysis {
    pub average_monthly_spending: f64,
    pub highest_spending_month: String,
    pub lowest_spending_month: String,
    pub spending_volatility: f64,
}

#[derive(Debug, Serialize)]
pub struct CostPerUserStats {
    pub average_cost_per_user: f64,
    pub median_cost_per_user: f64,
    pub top_spenders: Vec<UserSpending>,
    pub cost_distribution_by_user: Vec<UserCostCategory>,
}

#[derive(Debug, Serialize)]
pub struct UserSpending {
    pub user_id: String,
    pub email: String,
    pub total_spent: f64,
    pub average_per_entry: f64,
    pub entry_count: i32,
}

#[derive(Debug, Serialize)]
pub struct UserCostCategory {
    pub category: String,
    pub user_count: i32,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct UserBehaviorStats {
    pub activity_patterns: ActivityPatterns,
    pub engagement_metrics: EngagementMetrics,
    pub user_segments: Vec<UserSegment>,
    pub retention_analysis: RetentionAnalysis,
}

#[derive(Debug, Serialize)]
pub struct ActivityPatterns {
    pub most_active_users: Vec<UserActivity>,
    pub least_active_users: Vec<UserActivity>,
    pub activity_distribution: Vec<ActivityDistribution>,
    pub peak_usage_times: Vec<PeakUsageTime>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserActivity {
    pub user_id: String,
    pub email: String,
    pub entry_count: i32,
    pub days_active: i32,
    pub average_entries_per_day: f64,
    pub last_activity: String,
}

#[derive(Debug, Serialize)]
pub struct ActivityDistribution {
    pub activity_level: String,
    pub user_count: i32,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct PeakUsageTime {
    pub time_period: String,
    pub entry_count: i32,
    pub unique_users: i32,
}

#[derive(Debug, Serialize)]
pub struct EngagementMetrics {
    pub average_session_entries: f64,
    pub user_consistency_score: f64,
    pub feature_usage_stats: FeatureUsageStats,
}

#[derive(Debug, Serialize)]
pub struct FeatureUsageStats {
    pub odometer_usage_rate: f64,
    pub bulk_entry_usage: i32,
    pub average_entry_completeness: f64,
}

#[derive(Debug, Serialize)]
pub struct UserSegment {
    pub segment_name: String,
    pub user_count: i32,
    pub characteristics: String,
    pub average_spending: f64,
    pub average_entries: i32,
}

#[derive(Debug, Serialize)]
pub struct RetentionAnalysis {
    pub new_user_retention_7_day: f64,
    pub new_user_retention_30_day: f64,
    pub active_user_retention: f64,
    pub churn_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct PredictiveAnalytics {
    pub fuel_price_forecast: Vec<PriceForecast>,
    pub consumption_forecast: Vec<ConsumptionForecast>,
    pub user_growth_forecast: Vec<UserGrowthForecast>,
    pub revenue_projections: RevenueProjections,
}

#[derive(Debug, Serialize)]
pub struct PriceForecast {
    pub month: String,
    pub year: i32,
    pub predicted_price: f64,
    pub confidence_level: f64,
    pub trend_direction: String,
}

#[derive(Debug, Serialize)]
pub struct ConsumptionForecast {
    pub month: String,
    pub year: i32,
    pub predicted_consumption: f64,
    pub predicted_entries: i32,
}

#[derive(Debug, Serialize)]
pub struct UserGrowthForecast {
    pub month: String,
    pub year: i32,
    pub predicted_new_users: i32,
    pub predicted_total_users: i32,
    pub growth_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct RevenueProjections {
    pub next_month_revenue: f64,
    pub next_quarter_revenue: f64,
    pub annual_revenue_projection: f64,
    pub growth_assumptions: String,
}

#[derive(Debug, Serialize)]
pub struct PriceTrends {
    pub daily_price_trends: Vec<DailyPriceTrend>,
    pub price_volatility: PriceVolatility,
    pub price_comparisons: PriceComparisons,
    pub regional_price_data: Vec<RegionalPrice>,
}

#[derive(Debug, Serialize)]
pub struct DailyPriceTrend {
    pub date: String,
    pub average_price: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub price_range: f64,
}

#[derive(Debug, Serialize)]
pub struct PriceVolatility {
    pub volatility_index: f64,
    pub price_standard_deviation: f64,
    pub most_volatile_period: String,
    pub least_volatile_period: String,
}

#[derive(Debug, Serialize)]
pub struct PriceComparisons {
    pub current_vs_last_month: f64,
    pub current_vs_last_year: f64,
    pub lowest_recorded_price: f64,
    pub highest_recorded_price: f64,
    pub price_change_percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct RegionalPrice {
    pub region: String,
    pub average_price: f64,
    pub entry_count: i32,
    pub price_rank: i32,
}

#[derive(Debug, Deserialize)]
pub struct ServiceToggleRequest {
    pub service: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ServiceConfig {
    pub service_name: String,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub signin: bool,
    pub fuel_entry: bool,
}