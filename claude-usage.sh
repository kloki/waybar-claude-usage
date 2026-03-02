#!/bin/bash
token=$(jq -r '.claudeAiOauth.accessToken' ~/.claude/.credentials.json)

curl -s \
  -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $token" \
  -H "anthropic-beta: oauth-2025-04-20" \
  "https://api.anthropic.com/api/oauth/usage" | jq '{
    current_window: {
      utilization: (.five_hour.utilization | tostring + "%"),
      resets_at: .five_hour.resets_at
    },
    weekly_window: {
      utilization: (.seven_day.utilization | tostring + "%"),
      resets_at: .seven_day.resets_at
    },
    extra_usage: (
      if .extra_usage.is_enabled then {
        utilization: (.extra_usage.utilization | tostring + "%"),
        used: (.extra_usage.used_credits / 100 | "$\(.)"),
        limit: (.extra_usage.monthly_limit / 100 | "$\(.)")
      } else "disabled" end
    )
  }'
