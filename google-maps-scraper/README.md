# Google Maps Lead Scraper

A command-line tool that searches Google Maps, scrolls through the results,
and extracts business details into CSV and/or JSON. **No API key needed** —
it drives a real (automated) Chromium browser.

Built for lead generation: every place's web presence is classified, so you
can instantly filter for businesses that have **no real website** (nothing at
all, an Instagram/Facebook page, a hosted menu page, etc.) — your prospects.

## Extracted fields

| Field | Example / values |
|---|---|
| `name` | Casa Mantovani |
| `category` | Coffee shop |
| `status` | Operational / Temporarily closed / Permanently closed |
| `rating` | 4.9 |
| `reviews_count` | 144 |
| `price_level` | $$ |
| `address` | Old Souk Street, Jounieh, Lebanon |
| `phone` | +961 79 351 884 |
| `whatsapp_link` | https://wa.me/96179351884 (one-click outreach) |
| `website` | whatever Google lists, if anything |
| `website_type` | `none`, `instagram`, `facebook`, `whatsapp`, `linktree`, `tiktok`, `social`, `hosted-page`, `shortlink`, or `website` |
| `is_lead` | `yes` if they have no real website of their own |
| `emails` | scraped from their site with `--emails` |
| `plus_code`, `latitude`, `longitude` | location data |
| `opening_hours` | Monday: 7 AM–11 PM; ... |
| `google_maps_url` | link back to the place |

`hosted-page` means the "website" is a free builder or menu-hosting platform
(business.site, wixsite.com, omegasoftware menus, Zomato/TripAdvisor profiles,
etc.) — those businesses still count as leads.

## Setup

```bash
pip install -r requirements.txt
playwright install chromium
```

## Usage

```bash
# Basic: scrape up to 20 places, write results.csv + results.json
python scraper.py "restaurants in Beirut"

# Lead hunting: only keep places WITHOUT a real website
python scraper.py "barber shops in Jbeil" -n 40 --leads-only -o jbeil_barbers

# Also visit each business site and pull email addresses (slower)
python scraper.py "hotels in Batroun" -n 30 --emails

# Watch the browser while it works
python scraper.py "gyms near Jounieh" --headful
```

### Options

| Flag | Default | Description |
|---|---|---|
| `-n`, `--max-results` | 20 | Maximum number of places to scrape |
| `-o`, `--output` | `results` | Output basename (no extension) |
| `--format` | `both` | `csv`, `json`, or `both` |
| `--leads-only` | off | Keep only places without a real website |
| `--emails` | off | Visit business websites to extract emails |
| `--headful` | off | Show the browser window |
| `--lang` | `en` | Google Maps UI language |
| `--delay` | 1.0 | Seconds between place visits |
| `--retries` | 2 | Attempts per place before skipping |

## How it works

1. Opens `google.com/maps/search/<query>` in Chromium (Playwright) and
   dismisses the cookie-consent screen if shown.
2. Scrolls the results feed until it has enough unique places (deduplicated
   by Google's internal place ID) or hits the "end of the list" marker.
3. Visits each place page and reads the details panel; failed pages are
   retried before being skipped.
4. **Writes each row to the CSV immediately** — if the run is interrupted,
   everything scraped so far is already saved.
5. Classifies the web presence and flags leads; with `--emails` it also
   visits real websites (homepage + contact page) and extracts addresses,
   filtering out placeholder junk like `name@email.com`.
6. Prints a summary: how many places, how many leads, how many social-only.

## Example output

```
[scraper]   [1/25] Locanda A La Granda  [LEAD]
[scraper]   [2/25] Feniqia  [hosted-page]
[scraper]   [9/25] Ksar Lebanese Diner  [LEAD]
...
[scraper] Summary: 25 place(s) scraped — 19 lead(s) without a real website (7 of them social-media-only).
```

## Caveats

- **Terms of Service**: scraping Google Maps may violate Google's ToS. Use it
  responsibly and keep request volumes low (the built-in delay helps). For
  production-grade or high-volume use, the official
  [Places API](https://developers.google.com/maps/documentation/places/web-service)
  is the sanctioned route.
- Google changes its markup regularly. If a field starts coming back empty,
  the selectors at the top of `scraper.py` may need updating.
- Heavy use from one IP can trigger CAPTCHAs or temporary blocks. If that
  happens, wait a while, lower `-n`, and raise `--delay`.
