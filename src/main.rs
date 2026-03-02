use std::error::Error;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Default)]
struct WaybarModule {
    text: String,
    tooltip: String,
    class: String,
}

impl WaybarModule {
    pub fn new(text: String, tooltip: String, class: String) -> Self {
        Self {
            text,
            tooltip,
            class,
        }
    }
}

#[derive(Deserialize)]
struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OAuthToken,
}

#[derive(Deserialize)]
struct OAuthToken {
    #[serde(rename = "accessToken")]
    access_token: String,
}

#[derive(Deserialize)]
struct UsageResponse {
    five_hour: Window,
    seven_day: Window,
    extra_usage: ExtraUsage,
}

#[derive(Deserialize)]
struct Window {
    utilization: f64,
    resets_at: String,
}

#[derive(Deserialize)]
struct ExtraUsage {
    is_enabled: bool,
    utilization: Option<f64>,
    used_credits: Option<f64>,
    monthly_limit: Option<f64>,
}

fn make_bar(pct: f64) -> String {
    let filled = ((pct / 10.0).round() as usize).min(10);
    let empty = 10 - filled;
    format!("{}{}", "󰄮 ".repeat(filled), "󰄱 ".repeat(empty))
}

fn get_usage() -> Result<UsageResponse, Box<dyn Error>> {
    let home = std::env::var("HOME")?;
    let creds_path = format!("{home}/.claude/.credentials.json");
    let creds_text = std::fs::read_to_string(&creds_path)?;
    let creds: Credentials = serde_json::from_str(&creds_text)?;

    let resp = reqwest::blocking::Client::new()
        .get("https://api.anthropic.com/api/oauth/usage")
        .header(
            "Authorization",
            format!("Bearer {}", creds.claude_ai_oauth.access_token),
        )
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .send()?
        .error_for_status()?
        .json::<UsageResponse>()?;

    Ok(resp)
}

fn format_text(usage: &UsageResponse) -> String {
    format!("✻ {:.0}%", usage.five_hour.utilization)
}

fn format_resets_in(resets_at: &str) -> String {
    let Ok(reset_time) = resets_at.parse::<DateTime<Utc>>() else {
        return resets_at.to_string();
    };
    let duration = reset_time - Utc::now();
    let total_minutes = duration.num_minutes().max(0);
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours >= 24 {
        let days = hours / 24;
        let remaining_hours = hours % 24;
        format!("{days}d {remaining_hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn format_tooltip(usage: &UsageResponse) -> String {
    let mut lines = vec![
        format!(
            "5-hour  {}  {:.0}%",
            make_bar(usage.five_hour.utilization),
            usage.five_hour.utilization
        ),
        format!(
            "Resets  in {}",
            format_resets_in(&usage.five_hour.resets_at)
        ),
        format!(
            "7-day   {}  {:.0}%",
            make_bar(usage.seven_day.utilization),
            usage.seven_day.utilization
        ),
        format!(
            "Resets  in {}",
            format_resets_in(&usage.seven_day.resets_at)
        ),
    ];

    if usage.extra_usage.is_enabled {
        let pct = usage.extra_usage.utilization.unwrap_or(0.0);
        let used = usage.extra_usage.used_credits.unwrap_or(0.0) / 100.0;
        let limit = usage.extra_usage.monthly_limit.unwrap_or(0.0) / 100.0;
        lines.push(format!(
            "Extra   {}  {:.0}% (${used:.2}/${limit:.2})",
            make_bar(pct),
            pct
        ));
    } else {
        lines.push("Extra   disabled".to_string());
    }

    lines.join("\n")
}

fn build_module() -> Result<WaybarModule, Box<dyn Error>> {
    let usage = get_usage()?;
    let module = WaybarModule::new(
        format_text(&usage),
        format_tooltip(&usage),
        "claude".to_string(),
    );
    Ok(module)
}

fn main() {
    let module = build_module().unwrap_or_default();
    println!("{}", serde_json::to_string(&module).unwrap())
}
