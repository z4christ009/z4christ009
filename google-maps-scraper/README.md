# Maps Lead Center — Google Maps Lead Scraper

Find businesses that need a website. Searches Google Maps, extracts business
details, classifies each place's web presence, and flags the ones **without a
real website** — your leads. **No API key needed** — it drives a real
(automated) Chromium browser.

Comes with a **web control center** so you never have to touch the command
line: start scrapes, watch live progress, filter and search results, and
download CSV/JSON — all from your browser.

## Quick start — one click

| Your system | Do this |
|---|---|
| **Windows** | Double-click `start.bat` |
| **Mac / Linux** | Run `./start.sh` |

That's it. On first run it installs everything it needs (takes a minute or
two), then opens the control center at **http://localhost:8765**. Next runs
start instantly. Keep the window it opens running; close it (or press
Ctrl+C) to stop.

## The control center

- **New Scrape panel** — type a query like `restaurants in Batroun`, set how
  many places, pick a speed, tick *Leads only* and/or *Extract emails*, hit
  **Start**.
- **Live progress** — a progress bar, live counts, and an activity log while
  the scrape runs. Rows appear in the table as they're scraped.
- **Stat cards** — places scraped, leads, social-media-only, already-have-a-site.
- **Filters & search** — one click to see only leads, only places with no web
  presence at all, only social-media-only, etc. Search by name/address/category.
  Click column headers to sort.
- **One-click outreach** — every row has a WhatsApp button (wa.me link built
  from the phone number), plus Maps and website links.
- **Downloads** — CSV (for Excel/Sheets) and JSON per run.
- **Run history** — past runs are saved on disk and survive restarts. Stop a
  running job any time; everything scraped so far is kept.

## Speed modes

The scraper always runs **silently** (headless browser, nothing pops up).
Three speed modes control how aggressive it is — measured on the same
10-place query:

| Mode | Speed | What's different |
|---|---|---|
| ⚡ **Fast** | ~35–40 places/min | Blocks image/font downloads, shortest waits, skips opening hours, 1 attempt per place |
| ⚖ **Balanced** (default) | ~15 places/min | Blocks heavy downloads, full data including hours |
| 🐢 **Thorough** | ~10 places/min | No blocking, longest waits, 3 attempts per place — gentlest on Google, best for big runs |

Fast mode extracted identical names, phones, websites, ratings, and photos in
testing — the only field it skips is opening hours. Use Thorough if you start
seeing CAPTCHAs or missing fields.

## Extracted fields

| Field | Example / values |
|---|---|
| `name` | Casa Mantovani |
| `category` | Coffee shop |
| `status` | Operational / Temporarily closed / Permanently closed |
| `rating`, `reviews_count` | 4.9, 144 |
| `price_level` | $$ |
| `address` | Old Souk Street, Jounieh, Lebanon |
| `phone` | +961 79 351 884 |
| `whatsapp_link` | https://wa.me/96179351884 |
| `website` | whatever Google lists, if anything |
| `website_type` | `none`, `instagram`, `facebook`, `whatsapp`, `linktree`, `tiktok`, `social`, `hosted-page`, `shortlink`, or `website` |
| `is_lead` | `yes` if they have no real website of their own |
| `emails` | scraped from their site with the emails option |
| `plus_code`, `latitude`, `longitude` | location data |
| `opening_hours` | Monday: 7 AM–11 PM; ... |
| `image_url` | photo of the place |
| `google_maps_url` | link back to the place |

`hosted-page` means the "website" is a free builder or menu-hosting platform
(business.site, wixsite.com, hosted menus, Zomato/TripAdvisor profiles, etc.)
— those businesses still count as leads.

## Command line (optional)

The scraper also works standalone, without the control center:

```bash
python scraper.py "restaurants in Beirut"                          # basic
python scraper.py "barber shops in Jbeil" -n 40 --leads-only       # leads only
python scraper.py "hotels in Batroun" -n 30 --emails               # + emails
python scraper.py "pharmacies in Tripoli" -n 50 --speed fast       # 2-3x faster
```

Options: `-n/--max-results`, `-o/--output`, `--format csv|json|both`,
`--speed fast|balanced|thorough`, `--leads-only`, `--emails`, `--headful`,
`--lang`, `--delay`, `--retries`.

## Manual setup (if you don't use the start scripts)

```bash
pip install -r requirements.txt
playwright install chromium
python app.py        # control center at http://localhost:8765
```

## How it works

1. Opens `google.com/maps/search/<query>` in Chromium (Playwright) and
   dismisses the cookie-consent screen if shown.
2. Scrolls the results feed until it has enough unique places (deduplicated
   by Google's internal place ID) or hits the "end of the list" marker.
3. Visits each place page and reads the details panel; failed pages are
   retried before being skipped.
4. **Saves every row immediately** — an interrupted run keeps everything
   scraped so far.
5. Classifies the web presence and flags leads; with emails enabled it also
   visits real websites (homepage + contact page) and extracts addresses,
   filtering out placeholder junk like `name@email.com`.

## Caveats

- **Terms of Service**: scraping Google Maps may violate Google's ToS. Use it
  responsibly and keep request volumes low (the built-in delay helps). For
  production-grade or high-volume use, the official
  [Places API](https://developers.google.com/maps/documentation/places/web-service)
  is the sanctioned route.
- Google changes its markup regularly. If a field starts coming back empty,
  the selectors at the top of `scraper.py` may need updating.
- Heavy use from one IP can trigger CAPTCHAs or temporary blocks. If that
  happens, wait a while, lower the max places, and raise the delay.
