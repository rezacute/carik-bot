---
name: carik-quotes
description: Inspirational quotes and sayings database.
metadata:
  {
    "carik": { "emoji": "✨", "requires": {} },
    "openclaw": { "emoji": "✨", "requires": {} }
  }
---

# Quotes Skill for Carik Bot

Inspirational quotes for daily motivation.

## Categories

- **Motivation**: Push through challenges
- **Wisdom**: Life lessons
- **Javanese**: Traditional Javanese wisdom
- **Tech**: Programming and tech quotes

## Usage

```
User: /quote
Carik: "The only way to do great work is to love what you do." - Steve Jobs

User: /quote javanese
Carik: "Urip iku uruping tumindak." (Life is the light of your actions.)
```

## Javanese wisdom examples

- **"Sapa nandur bakal ngunduh"** - Whoever plants will harvest
- **"Ala lan becik iku sakabehane"** - Bad and good is everything's nature
- **"Kathekan kridha ing prakara"** - Be patient in matters

## Adding new quotes

Quotes are stored in `skills/quotes/quotes.json`:

```json
{
  "category": "wisdom",
  "text": "Your quote here",
  "author": "Author name"
}
```
