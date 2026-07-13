#!/usr/bin/env python3
"""Google Maps scraper.

Searches Google Maps for a query, scrolls through the results feed, opens each
place, and extracts business details into CSV and/or JSON.

Usage:
    python scraper.py "restaurants in Beirut" --max-results 20 --output results
    python scraper.py "coffee shops in Jounieh" -n 10 --format json --headful

Extracted fields per place:
    name, category, rating, reviews_count, price_level, address, phone,
    website, plus_code, latitude, longitude, opening_hours, google_maps_url

Notes:
    - Requires: pip install playwright && playwright install chromium
    - Scraping Google Maps may violate Google's Terms of Service. Use
      responsibly, keep volumes low, and consider the official Places API
      for production use.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
import time
import urllib.parse
from dataclasses import dataclass, asdict, field

from playwright.sync_api import sync_playwright, Page, TimeoutError as PWTimeout

FEED_SELECTOR = 'div[role="feed"]'
RESULT_LINK_SELECTOR = 'a[href*="/maps/place/"]'

FIELDNAMES = [
    "name", "category", "rating", "reviews_count", "price_level", "address",
    "phone", "website", "plus_code", "latitude", "longitude",
    "opening_hours", "google_maps_url",
]


@dataclass
class Place:
    name: str = ""
    category: str = ""
    rating: str = ""
    reviews_count: str = ""
    price_level: str = ""
    address: str = ""
    phone: str = ""
    website: str = ""
    plus_code: str = ""
    latitude: str = ""
    longitude: str = ""
    opening_hours: dict = field(default_factory=dict)
    google_maps_url: str = ""


def log(msg: str) -> None:
    print(f"[scraper] {msg}", flush=True)


def accept_consent(page: Page) -> None:
    """Dismiss the Google cookie-consent interstitial if it appears."""
    try:
        for label in ("Accept all", "Reject all", "I agree"):
            btn = page.locator(f'button:has-text("{label}")').first
            if btn.is_visible(timeout=2000):
                btn.click()
                page.wait_for_load_state("domcontentloaded")
                return
    except Exception:
        pass


def collect_result_urls(page: Page, query: str, max_results: int) -> list[str]:
    """Search and scroll the results feed until enough place URLs are found."""
    url = "https://www.google.com/maps/search/" + urllib.parse.quote(query)
    page.goto(url, wait_until="domcontentloaded", timeout=60000)
    accept_consent(page)

    # A direct hit on a single place redirects to /maps/place/ with no feed.
    page.wait_for_timeout(3000)
    if "/maps/place/" in page.url:
        return [page.url]

    try:
        page.wait_for_selector(FEED_SELECTOR, timeout=15000)
    except PWTimeout:
        log("No results feed found; the query may have returned nothing.")
        return []

    urls: list[str] = []
    seen: set[str] = set()
    stale_rounds = 0

    while len(urls) < max_results and stale_rounds < 6:
        for el in page.locator(f"{FEED_SELECTOR} {RESULT_LINK_SELECTOR}").all():
            href = el.get_attribute("href") or ""
            key = href.split("?")[0]
            if key and key not in seen:
                seen.add(key)
                urls.append(href)
        if len(urls) >= max_results:
            break

        before = len(urls)
        page.locator(FEED_SELECTOR).evaluate("el => el.scrollBy(0, 2000)")
        page.wait_for_timeout(1500)

        # "You've reached the end of the list." sentinel
        if page.locator(f'{FEED_SELECTOR} >> text=/reached the end/i').count():
            for el in page.locator(f"{FEED_SELECTOR} {RESULT_LINK_SELECTOR}").all():
                href = el.get_attribute("href") or ""
                key = href.split("?")[0]
                if key and key not in seen:
                    seen.add(key)
                    urls.append(href)
            break

        stale_rounds = stale_rounds + 1 if len(urls) == before else 0

    return urls[:max_results]


def parse_coords_from_url(url: str) -> tuple[str, str]:
    m = re.search(r"!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)", url)
    if m:
        return m.group(1), m.group(2)
    m = re.search(r"@(-?\d+\.\d+),(-?\d+\.\d+)", url)
    if m:
        return m.group(1), m.group(2)
    return "", ""


def text_or_empty(page: Page, selector: str) -> str:
    loc = page.locator(selector).first
    try:
        if loc.count():
            return (loc.inner_text(timeout=2000) or "").strip()
    except Exception:
        pass
    return ""


def attr_or_empty(page: Page, selector: str, attr: str) -> str:
    loc = page.locator(selector).first
    try:
        if loc.count():
            return (loc.get_attribute(attr, timeout=2000) or "").strip()
    except Exception:
        pass
    return ""


def scrape_place(page: Page, url: str) -> Place:
    page.goto(url, wait_until="domcontentloaded", timeout=60000)
    page.wait_for_timeout(2500)

    p = Place(google_maps_url=page.url.split("?")[0])
    p.latitude, p.longitude = parse_coords_from_url(page.url)

    p.name = text_or_empty(page, "h1")
    p.category = text_or_empty(page, 'button[jsaction*="category"]')

    # Rating and review count live in the header area, e.g. "4.6" and "(1,234)"
    rating_txt = attr_or_empty(page, 'div[role="main"] span[role="img"]', "aria-label")
    m = re.search(r"(\d+(?:\.\d+)?)", rating_txt)
    if m:
        p.rating = m.group(1)
    reviews_txt = text_or_empty(page, 'div[role="main"] span[aria-label*="review"]') or \
        text_or_empty(page, 'div[role="main"] button[jsaction*="reviewChart"]')
    m = re.search(r"([\d,.]+)", reviews_txt)
    if m:
        p.reviews_count = m.group(1).replace(",", "")

    price_txt = attr_or_empty(page, 'span[aria-label*="Price"]', "aria-label")
    m = re.search(r"Price:\s*(.+)", price_txt)
    if m:
        p.price_level = m.group(1).strip()

    # Detail rows are buttons/links with data-item-id attributes.
    p.address = attr_or_empty(page, 'button[data-item-id="address"]', "aria-label")
    p.address = re.sub(r"^Address:\s*", "", p.address)

    phone = attr_or_empty(page, 'button[data-item-id^="phone"]', "aria-label")
    p.phone = re.sub(r"^Phone:\s*", "", phone)

    p.website = attr_or_empty(page, 'a[data-item-id="authority"]', "href")

    plus_code = attr_or_empty(page, 'button[data-item-id="oloc"]', "aria-label")
    p.plus_code = re.sub(r"^Plus code:\s*", "", plus_code)

    # Opening hours: expand the hours section if present, then read the table.
    try:
        hours_toggle = page.locator(
            'div[role="main"] [jsaction*="openhours"], '
            'div[role="main"] img[aria-label*="Hours"]'
        ).first
        if hours_toggle.count():
            hours_toggle.click(timeout=2000)
            page.wait_for_timeout(800)
    except Exception:
        pass
    try:
        for row in page.locator("table tr").all():
            cells = row.locator("td, th").all_inner_texts()
            if len(cells) >= 2:
                day = cells[0].strip().split("\n")[0]
                hours = cells[1].strip().split("\n")[0]
                if day and hours:
                    p.opening_hours[day] = hours
    except Exception:
        pass

    return p


def write_csv(places: list[Place], path: str) -> None:
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=FIELDNAMES)
        w.writeheader()
        for p in places:
            row = asdict(p)
            row["opening_hours"] = "; ".join(
                f"{d}: {h}" for d, h in p.opening_hours.items()
            )
            w.writerow(row)


def write_json(places: list[Place], path: str) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump([asdict(p) for p in places], f, ensure_ascii=False, indent=2)


def main() -> int:
    ap = argparse.ArgumentParser(description="Scrape Google Maps search results.")
    ap.add_argument("query", help='Search query, e.g. "restaurants in Beirut"')
    ap.add_argument("-n", "--max-results", type=int, default=20,
                    help="Maximum number of places to scrape (default 20)")
    ap.add_argument("-o", "--output", default="results",
                    help="Output file basename without extension (default: results)")
    ap.add_argument("--format", choices=["csv", "json", "both"], default="both",
                    help="Output format (default: both)")
    ap.add_argument("--headful", action="store_true",
                    help="Run with a visible browser window")
    ap.add_argument("--lang", default="en", help="UI language (default: en)")
    ap.add_argument("--delay", type=float, default=1.0,
                    help="Delay in seconds between place visits (default 1.0)")
    args = ap.parse_args()

    with sync_playwright() as pw:
        browser = pw.chromium.launch(
            headless=not args.headful,
            args=["--disable-blink-features=AutomationControlled", "--lang=" + args.lang],
        )
        context = browser.new_context(
            viewport={"width": 1440, "height": 900},
            locale=args.lang,
            user_agent=(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
            ),
        )
        page = context.new_page()

        log(f"Searching for: {args.query!r}")
        urls = collect_result_urls(page, args.query, args.max_results)
        log(f"Found {len(urls)} result link(s). Scraping details...")

        places: list[Place] = []
        for i, url in enumerate(urls, 1):
            try:
                place = scrape_place(page, url)
                places.append(place)
                log(f"  [{i}/{len(urls)}] {place.name or '(unnamed)'}")
            except Exception as e:
                log(f"  [{i}/{len(urls)}] failed: {e}")
            time.sleep(args.delay)

        browser.close()

    if not places:
        log("No places scraped.")
        return 1

    if args.format in ("csv", "both"):
        write_csv(places, args.output + ".csv")
        log(f"Wrote {args.output}.csv")
    if args.format in ("json", "both"):
        write_json(places, args.output + ".json")
        log(f"Wrote {args.output}.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
