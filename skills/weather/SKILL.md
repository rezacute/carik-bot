---
name: carik-weather
description: Get current weather and forecasts using wttr.in (no API key required).
metadata:
  {
    "carik": { "emoji": "🌤️", "requires": { "bins": ["curl"] } },
    "openclaw": { "emoji": "🌤️", "requires": { "bins": ["curl"] } }
  }
---

# Weather Skill for Carik Bot

Two free services, no API keys needed.

## wttr.in (primary)

Quick one-liner format:

```bash
curl -s "wttr.in/London?format=3"
# Output: London: ⛅️ +8°C
```

Compact format:

```bash
curl -s "wttr.in/London?format=%l:+%c+%t+%h+%w"
# Output: London: ⛅️ +8°C 71% ↙5km/h
```

## Format codes

| Code | Meaning |
|------|---------|
| `%c` | condition |
| `%t` | temperature |
| `%h` | humidity |
| `%w` | wind |
| `%l` | location |
| `%m` | moon phase |

## Usage tips

- URL-encode spaces: `wttr.in/New+York`
- Airport codes: `wttr.in/JFK`
- Units: `?m` (metric) `?u` (USCS)
- Today only: `?1` · Current only: `?0`

## Example usage

```
User: Weather in Jakarta
Carik: Jakarta: 🌧️ +28°C 80% ↗15km/h
```
