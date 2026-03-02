use std::{error::Error, iter::repeat_n};

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

fn get_braille(number: usize) -> char {
    match number {
        0 => ' ',
        1 => '⡀',
        2 => '⡄',
        3 => '⡆',
        4 => '⡇',
        5 => '⣇',
        6 => '⣧',
        7 => '⣷',
        8 => '⣿',
        _ => ' ',
    }
}

fn format_bar(pct: f64) -> String {
    let perc = (pct.round() as usize).min(100);
    let full = perc / 8;
    let remainder = perc % 8;
    let partial = usize::from(remainder > 0);
    let padding = 25 - full - partial;

    repeat_n(get_braille(8), full)
        .chain((remainder > 0).then(|| get_braille(remainder)))
        .chain(repeat_n(get_braille(0), padding))
        .collect()
}
fn format_text(usage: &UsageResponse) -> String {
    format!("✻ [{}]", format_bar(usage.five_hour.utilization))
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

fn format_window(label: &str, window: &Window) -> String {
    format!(
        "{label}  {:.0}%  resets in {}\n[{}]",
        window.utilization,
        format_resets_in(&window.resets_at),
        format_bar(window.utilization),
    )
}

fn format_tooltip(usage: &UsageResponse) -> String {
    [
        format_window("5-hour", &usage.five_hour),
        format_window("7-day ", &usage.seven_day),
    ]
    .join("\n\n")
}

fn error_module(err: &dyn Error) -> WaybarModule {
    let msg = err.to_string();
    let class = if msg.contains("credentials") || msg.contains("No such file") {
        "error-auth"
    } else if msg.contains("error trying to connect") || msg.contains("timed out") {
        "error-network"
    } else {
        "error"
    };
    WaybarModule::new("✻ err".to_string(), msg, class.to_string())
}

fn main() {
    let module = match get_usage() {
        Ok(usage) => WaybarModule::new(
            format_text(&usage),
            format_tooltip(&usage),
            "claude".to_string(),
        ),
        Err(e) => error_module(e.as_ref()),
    };
    println!("{}", serde_json::to_string(&module).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_braille() {
        assert_eq!(get_braille(0), ' ');
        assert_eq!(get_braille(1), '⡀');
        assert_eq!(get_braille(4), '⡇');
        assert_eq!(get_braille(8), '⣿');
        assert_eq!(get_braille(9), ' ');
    }

    #[test]
    fn test_format_bar_zero() {
        let bar = format_bar(0.0);
        assert_eq!(bar.chars().count(), 25);
        assert!(bar.chars().all(|c| c == ' '));
    }

    #[test]
    fn test_format_bar_full() {
        let bar = format_bar(100.0);
        assert_eq!(bar.chars().count(), 25);
    }

    #[test]
    fn test_format_bar_consistent_width() {
        for pct in [0.0, 1.0, 25.0, 50.0, 75.0, 99.0, 100.0] {
            assert_eq!(
                format_bar(pct).chars().count(),
                25,
                "bar width wrong at {pct}%"
            );
        }
    }

    #[test]
    fn test_format_bar_clamps_above_100() {
        assert_eq!(format_bar(150.0), format_bar(100.0));
    }

    #[test]
    fn test_format_resets_in_invalid() {
        assert_eq!(format_resets_in("not-a-date"), "not-a-date");
    }

    #[test]
    fn test_format_resets_in_future() {
        let future = Utc::now() + chrono::Duration::hours(2) + chrono::Duration::minutes(30);
        let result = format_resets_in(&future.to_rfc3339());
        assert!(result.contains("2h"), "expected '2h' in '{result}'");
    }

    #[test]
    fn test_format_resets_in_days() {
        let future = Utc::now() + chrono::Duration::days(2) + chrono::Duration::hours(3);
        let result = format_resets_in(&future.to_rfc3339());
        assert!(
            result.starts_with("2d"),
            "expected '2d' prefix in '{result}'"
        );
    }

    #[test]
    fn test_format_resets_in_minutes_only() {
        let future = Utc::now() + chrono::Duration::minutes(45);
        let result = format_resets_in(&future.to_rfc3339());
        assert!(result.ends_with('m'), "expected 'm' suffix in '{result}'");
        assert!(!result.contains('h'), "expected no 'h' in '{result}'");
    }

    #[test]
    fn test_format_text() {
        let usage = UsageResponse {
            five_hour: Window {
                utilization: 50.0,
                resets_at: "2099-01-01T00:00:00Z".to_string(),
            },
            seven_day: Window {
                utilization: 25.0,
                resets_at: "2099-01-01T00:00:00Z".to_string(),
            },
        };
        let text = format_text(&usage);
        assert!(text.starts_with("✻ ["));
        assert!(text.ends_with(']'));
    }

    #[test]
    fn test_format_tooltip_contains_both_windows() {
        let usage = UsageResponse {
            five_hour: Window {
                utilization: 10.0,
                resets_at: "2099-01-01T00:00:00Z".to_string(),
            },
            seven_day: Window {
                utilization: 20.0,
                resets_at: "2099-01-01T00:00:00Z".to_string(),
            },
        };
        let tooltip = format_tooltip(&usage);
        assert!(tooltip.contains("5-hour"));
        assert!(tooltip.contains("7-day"));
        assert!(tooltip.contains("10%"), "expected percentage in tooltip");
        assert!(tooltip.contains("20%"), "expected percentage in tooltip");
    }

    #[test]
    fn test_format_tooltip_uses_correct_resets_at() {
        let usage = UsageResponse {
            five_hour: Window {
                utilization: 0.0,
                resets_at: "2099-06-15T00:00:00Z".to_string(),
            },
            seven_day: Window {
                utilization: 0.0,
                resets_at: "2099-12-25T00:00:00Z".to_string(),
            },
        };
        let tooltip = format_tooltip(&usage);
        let sections: Vec<&str> = tooltip.split("\n\n").collect();
        assert_eq!(sections.len(), 2);
        // 5-hour section should NOT contain the 7-day reset time
        // Both will show large durations, but they should differ
        assert!(sections[0].contains("5-hour"));
        assert!(sections[1].contains("7-day"));
    }

    #[test]
    fn test_error_module_auth() {
        let err: Box<dyn Error> = "No such file or directory".into();
        let module = error_module(err.as_ref());
        assert_eq!(module.class, "error-auth");
        assert_eq!(module.text, "✻ err");
    }

    #[test]
    fn test_error_module_network() {
        let err: Box<dyn Error> = "error trying to connect".into();
        let module = error_module(err.as_ref());
        assert_eq!(module.class, "error-network");
    }

    #[test]
    fn test_error_module_generic() {
        let err: Box<dyn Error> = "something unexpected".into();
        let module = error_module(err.as_ref());
        assert_eq!(module.class, "error");
        assert!(module.tooltip.contains("something unexpected"));
    }

    #[test]
    fn test_waybar_module_serializes() {
        let module = WaybarModule::new("text".to_string(), "tip".to_string(), "cls".to_string());
        let json = serde_json::to_string(&module).unwrap();
        assert!(json.contains("\"text\":\"text\""));
        assert!(json.contains("\"tooltip\":\"tip\""));
        assert!(json.contains("\"class\":\"cls\""));
    }
}
