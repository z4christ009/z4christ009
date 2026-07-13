# Google Maps Scraper

A command-line tool that searches Google Maps, scrolls through the results, and
extracts business details into CSV and/or JSON.

## Extracted fields

| Field | Example |
|---|---|
| `name` | Casa Mantovani |
| `category` | Coffee shop |
| `rating` | 4.9 |
| `reviews_count` | 144 |
| `price_level` | $$ |
| `address` | Old Souk Street, Jounieh, Lebanon |
| `phone` | +961 79 351 884 |
| `website` | https://example.com |
| `plus_code` | XJPP+MF Jounieh, Lebanon |
| `latitude` / `longitude` | 33.9867403 / 35.636216 |
| `opening_hours` | Monday: 7 AM–11 PM, ... |
| `google_maps_url` | link to the place |

## Setup

```bash
pip install -r requirements.txt
playwright install chromium
```

## Usage

```bash
# Basic: scrape up to 20 places, write results.csv + results.json
python scraper.py "restaurants in Beirut"

# 50 results, JSON only, custom output name
python scraper.py "pharmacies in Tripoli Lebanon" -n 50 --format json -o pharmacies

# Watch the browser while it works
python scraper.py "gyms near Jounieh" --headful
```

### Options

| Flag | Default | Description |
|---|---|---|
| `-n`, `--max-results` | 20 | Maximum number of places to scrape |
| `-o`, `--output` | `results` | Output basename (no extension) |
| `--format` | `both` | `csv`, `json`, or `both` |
| `--headful` | off | Show the browser window |
| `--lang` | `en` | Google Maps UI language |
| `--delay` | 1.0 | Seconds to wait between place visits |

## How it works

1. Opens `google.com/maps/search/<query>` in a Chromium browser (Playwright).
2. Dismisses the cookie-consent screen if shown.
3. Scrolls the results feed until it has enough place links (or hits the
   "end of the list" marker).
4. Visits each place page and reads the details panel (name, rating, address,
   phone, website, hours, coordinates from the URL, etc.).
5. Writes the results to `<output>.csv` and/or `<output>.json`.

## Caveats

- **Terms of Service**: scraping Google Maps may violate Google's ToS. Use it
  responsibly, keep request volumes low (the built-in delay helps), and use the
  official [Places API](https://developers.google.com/maps/documentation/places/web-service)
  for anything production-grade or high-volume.
- Google changes its markup regularly. If a field starts coming back empty,
  the selectors in `scraper.py` may need updating.
- Heavy use from one IP can trigger CAPTCHAs or temporary blocks.
