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
}

#[derive(Deserialize)]
struct Window {
    utilization: f64,
    resets_at: String,
}

fn make_grid(pct: f64) -> String {
    let filled = (pct.round() as usize).min(100);
    let mut rows = Vec::new();
    for row in 0..10 {
        let mut cells = Vec::new();
        for col in 0..10 {
            let idx = row * 10 + col;
            if idx < filled {
                cells.push("󰄮");
            } else {
                cells.push("󰄱");
            }
        }
        rows.push(cells.join(" "));
    }
    rows.join("\n")
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
    let mut sections = Vec::new();

    sections.push(format!(
        "5-hour  {:.0}%  (resets in {})\n{}",
        usage.five_hour.utilization,
        format_resets_in(&usage.five_hour.resets_at),
        make_grid(usage.five_hour.utilization),
    ));

    sections.push(format!(
        "7-day  {:.0}%  (resets in {})\n{}",
        usage.seven_day.utilization,
        format_resets_in(&usage.seven_day.resets_at),
        make_grid(usage.seven_day.utilization),
    ));

    sections.join("\n\n")
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
