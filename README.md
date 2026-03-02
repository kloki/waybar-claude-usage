# waybar-claude-usage

A Claude Code usage module for [waybar](https://github.com/Alexays/Waybar) that works for me.

# Install

```bash
cargo install waybar-claude-usage
```

## Binaries

Check [Releases](https://github.com/kloki/waybar-claude-usage/releases) for binaries and installers

# Configure

Add this to your `config.jsonc`

```json
{
  "custom/claude": {
    "exec": "~/.cargo/bin/waybar-claude-usage",
    "return-type": "json",
    "interval": 120
  }
}
```
