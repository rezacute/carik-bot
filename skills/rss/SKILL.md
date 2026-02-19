---
name: carik-rss
description: Fetch and summarize RSS news feeds. Supports multiple sources and can detect topics.
metadata:
  {
    "carik": { "emoji": "📰", "requires": { "features": ["llm", "rss"] } },
    "openclaw": { "emoji": "📰", "requires": { "features": ["llm", "rss"] } }
  }
---

# RSS News Skill for Carik Bot

Fetch latest news from RSS feeds and use LLM to summarize.

## Supported Sources

| Source | Keywords | RSS URL |
|--------|----------|---------|
| Yahoo News | yahoo, yahoo news | https://news.yahoo.com/rss/topstories |
| Google News | google, google news | https://news.google.com/rss |
| BBC News | bbc, bbc news | http://feeds.bbci.co.uk/news/rss.xml |
| BBC World | bbc world | http://feeds.bbci.co.uk/news/world/rss.xml |
| TechCrunch | techcrunch | https://techcrunch.com/feed/ |
| Hacker News | hn, hacker news, hackernews | https://hnrss.org/newest |
| CNA | cna, channel newsasia, channelnewsasia | https://www.channelnewsasia.com/rss |
| Reuters | reuters | https://www.reutersagency.com/feed/ |

## Topic Detection with Specific RSS Sources (G20)

When you ask about a specific country/topic, the bot uses country-specific RSS feeds:

| Country | Keywords | RSS Source |
|---------|----------|-------------|
| Argentina | argentina | Buenos Aires Times |
| Australia | australia, australian | ABC News |
| Brazil | brazil, brazilian | Agência Brasil |
| Canada | canada, canadian | CBC News |
| China | china, chinese | SCMP |
| France | france, french | France 24 |
| Germany | germany, german | Deutsche Welle |
| India | india, indian | Times of India |
| Indonesia | indonesia, indonesian | Antara News |
| Italy | italy, italian | ANSA |
| Japan | japan, japanese | Japan Times |
| Mexico | mexico, mexican | El Universal |
| Russia | russia, russian | RT News |
| Saudi Arabia | saudi, saudia | Arab News |
| South Africa | south africa | Mail & Guardian |
| South Korea | korea, korean | Yonhap News |
| Turkey | turkey, turkish | TRT World |
| UK | uk, britain, british | BBC News |
| USA | us, usa, america | NY Times |
| EU | eu, europe | Euronews |
| Singapore | singapore, singaporean | CNA |

## How It Works

1. Detect if message is news-related (news, headlines, latest, feed, berita)
2. Detect topic (country or subject)
3. Detect source if specified
4. Fetch RSS feed
5. **Use LLM to summarize** the headlines into friendly format

## Example Usage

```
User: What's the latest news?
Carik: Here's the latest news:
• Trump administration slams New York Fed study...
• Delta flight makes emergency landing...
• Plus 3 more stories...

User: News about India
Carik: Here are the latest India news:
• [Headlines from Yahoo News - LLM summarizes relevant ones]

User: TechCrunch
Carik: Here's the latest from TechCrunch:
• [Tech news headlines summarized]
```

## Response Format

LLM summarization makes responses:
- More conversational
- Filter out irrelevant headlines
- Group related stories
- Highlight key points
