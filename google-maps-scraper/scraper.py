#!/usr/bin/env python3
"""Google Maps lead scraper — no API key needed.

Searches Google Maps in a real (automated) browser, scrolls through the
results, opens each place, and extracts business details. Built for lead
generation: it classifies each place's web presence so you can instantly see
who has no real website (social-media-only, hosted menu page, or nothing).

Can be used two ways:
  1. CLI:            python scraper.py "restaurants in Beirut" -n 40
  2. Programmatic:   from scraper import run_scrape  (used by the control
                     center in app.py)

Requires:
    pip install playwright
    playwright install chromium

Note: scraping Google Maps may violate Google's Terms of Service. Keep
volumes reasonable; the built-in delays help you stay polite.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import sys
import time
import urllib.parse
from dataclasses import dataclass, asdict, field
from typing import Callable, Optional

from playwright.sync_api import sync_playwright, Page, TimeoutError as PWTimeout

FEED_SELECTOR = 'div[role="feed"]'
RESULT_LINK_SELECTOR = 'a[href*="/maps/place/"]'

SOCIAL_DOMAINS = {
    "instagram.com": "instagram",
    "instagr.am": "instagram",
    "facebook.com": "facebook",
    "fb.com": "facebook",
    "fb.me": "facebook",
    "m.me": "facebook",
    "wa.me": "whatsapp",
    "api.whatsapp.com": "whatsapp",
    "linktr.ee": "linktree",
    "beacons.ai": "linktree",
    "taplink.cc": "linktree",
    "tiktok.com": "tiktok",
    "twitter.com": "social",
    "x.com": "social",
    "youtube.com": "social",
}

# Free site builders, hosted menu pages, and aggregator profiles. A business
# whose only web presence lives on one of these has no real website of its
# own — exactly the leads we want to flag.
HOSTED_DOMAINS = (
    "business.site", "godaddysites.com", "wixsite.com", "wix.com",
    "square.site", "weebly.com", "wordpress.com", "blogspot.com",
    "sites.google.com", "webnode.com", "webs.com", "mystrikingly.com",
    "carrd.co", "jimdosite.com",
    # menu-hosting / ordering / aggregator platforms
    "omegasoftware.ca", "finedinemenu.com", "menulist.menu", "ordable.com",
    "zomato.com", "talabat.com", "toasttab.com", "popmenu.com",
    "untappd.com", "yelp.com", "tripadvisor.com",
)

SHORTLINK_DOMAINS = ("bit.ly", "goo.gl", "tinyurl.com", "t.co", "cutt.ly")

FIELDNAMES = [
    "name", "category", "status", "rating", "reviews_count", "price_level",
    "address", "phone", "whatsapp_link", "website", "website_type", "is_lead",
    "emails", "plus_code", "latitude", "longitude", "opening_hours",
    "image_url", "google_maps_url",
]

EMAIL_RE = re.compile(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
EMAIL_JUNK = re.compile(
    r"\.(png|jpe?g|gif|svg|webp|css|js)$"
    r"|@(example|sentry|schema|wixpress|placeholder|domain|yourdomain"
    r"|email|company|website|mysite|test|sample)\."
    r"|^(name|your ?name|firstname|lastname|email|youremail|user|test|info"
    r"|john\.?doe|jane\.?doe|someone|example|abc|xyz)@",
    re.I,
)

# The stable place identifier inside a Google Maps URL, e.g.
# "!1s0x151f410033f2042f:0xf8cfc0d70f385613". Two different links to the same
# place (ads, re-ranked results) share this ID, so it is the dedup key.
PLACE_ID_RE = re.compile(r"!1s(0x[0-9a-f]+:0x[0-9a-f]+)", re.I)


@dataclass
class Place:
    name: str = ""
    category: str = ""
    status: str = ""
    rating: str = ""
    reviews_count: str = ""
    price_level: str = ""
    address: str = ""
    phone: str = ""
    whatsapp_link: str = ""      # wa.me link built from the phone number
    website: str = ""
    # none | instagram | facebook | whatsapp | linktree | tiktok | social |
    # hosted-page | shortlink | website
    website_type: str = "none"
    is_lead: str = "yes"         # yes unless website_type == "website"
    emails: str = ""
    plus_code: str = ""
    latitude: str = ""
    longitude: str = ""
    opening_hours: dict = field(default_factory=dict)
    image_url: str = ""
    google_maps_url: str = ""


def classify_website(url: str) -> str:
    if not url:
        return "none"
    try:
        host = urllib.parse.urlparse(url).netloc.lower()
    except Exception:
        return "website"
    host = host[4:] if host.startswith("www.") else host

    def matches(domain: str) -> bool:
        return host == domain or host.endswith("." + domain)

    for domain, kind in SOCIAL_DOMAINS.items():
        if matches(domain):
            return kind
    for domain in HOSTED_DOMAINS:
        if matches(domain):
            return "hosted-page"
    for domain in SHORTLINK_DOMAINS:
        if matches(domain):
            return "shortlink"
    return "website"


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


def place_row(p: Place) -> dict:
    row = asdict(p)
    row["opening_hours"] = "; ".join(f"{d}: {h}" for d, h in p.opening_hours.items())
    return row


class IncrementalCSV:
    """Appends rows as they are scraped so a crash never loses progress."""

    def __init__(self, path: str):
        self.path = path
        new_file = not os.path.exists(path) or os.path.getsize(path) == 0
        self.f = open(path, "a", newline="", encoding="utf-8")
        self.w = csv.DictWriter(self.f, fieldnames=FIELDNAMES)
        if new_file:
            self.w.writeheader()
            self.f.flush()

    def write(self, p: Place) -> None:
        self.w.writerow(place_row(p))
        self.f.flush()

    def close(self) -> None:
        self.f.close()


class Scraper:
    """Drives one browser session. Reusable across queries."""

    def __init__(self, headless: bool = True, lang: str = "en"):
        self._pw = sync_playwright().start()
        self.browser = self._pw.chromium.launch(
            headless=headless,
            args=["--disable-blink-features=AutomationControlled",
                  "--lang=" + lang],
        )
        self.context = self.browser.new_context(
            viewport={"width": 1440, "height": 900},
            locale=lang,
            user_agent=(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
            ),
        )
        self.page = self.context.new_page()
        self.email_page: Optional[Page] = None

    def close(self) -> None:
        try:
            self.browser.close()
        finally:
            self._pw.stop()

    # ---- result collection -------------------------------------------------

    def collect_result_urls(self, query: str, max_results: int,
                            log: Callable[[str], None],
                            should_stop: Callable[[], bool]) -> list[str]:
        page = self.page
        url = "https://www.google.com/maps/search/" + urllib.parse.quote(query)
        page.goto(url, wait_until="domcontentloaded", timeout=60000)
        accept_consent(page)

        # A direct hit on a single place redirects to /maps/place/, no feed.
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

        def harvest() -> None:
            for el in page.locator(
                    f"{FEED_SELECTOR} {RESULT_LINK_SELECTOR}").all():
                href = el.get_attribute("href") or ""
                if not href:
                    continue
                m = PLACE_ID_RE.search(href)
                key = m.group(1) if m else href.split("?")[0]
                if key not in seen:
                    seen.add(key)
                    urls.append(href)

        while len(urls) < max_results and stale_rounds < 8:
            if should_stop():
                break
            harvest()
            if len(urls) >= max_results:
                break

            before = len(urls)
            page.locator(FEED_SELECTOR).evaluate(
                "el => el.scrollBy(0, el.scrollHeight)")
            page.wait_for_timeout(1600)

            # "You've reached the end of the list." sentinel
            if page.locator(
                    f'{FEED_SELECTOR} >> text=/reached the end/i').count():
                harvest()
                log("Reached the end of the results list.")
                break

            stale_rounds = stale_rounds + 1 if len(urls) == before else 0

        return urls[:max_results]

    # ---- single place ------------------------------------------------------

    def scrape_place(self, url: str) -> Place:
        page = self.page
        page.goto(url, wait_until="domcontentloaded", timeout=60000)
        page.wait_for_selector('div[role="main"] h1', timeout=15000)
        page.wait_for_timeout(1200)

        p = Place(google_maps_url=page.url.split("?")[0])
        p.latitude, p.longitude = parse_coords_from_url(page.url)

        p.name = text_or_empty(page, 'div[role="main"] h1')
        p.category = text_or_empty(page, 'button[jsaction*="category"]')

        for status_pat in ("Permanently closed", "Temporarily closed"):
            if page.locator(
                    f'div[role="main"] >> text="{status_pat}"').count():
                p.status = status_pat
                break
        else:
            p.status = "Operational"

        rating_txt = attr_or_empty(
            page, 'div[role="main"] span[role="img"]', "aria-label")
        m = re.search(r"(\d+(?:\.\d+)?)", rating_txt)
        if m:
            p.rating = m.group(1)
        reviews_txt = text_or_empty(
            page, 'div[role="main"] span[aria-label*="review"]') or \
            text_or_empty(
                page, 'div[role="main"] button[jsaction*="reviewChart"]')
        m = re.search(r"([\d,.]+)", reviews_txt)
        if m:
            p.reviews_count = m.group(1).replace(",", "")

        price_txt = attr_or_empty(page, 'span[aria-label*="Price"]', "aria-label")
        m = re.search(r"Price:\s*(.+)", price_txt)
        if m:
            p.price_level = m.group(1).strip()

        p.address = re.sub(
            r"^Address:\s*", "",
            attr_or_empty(page, 'button[data-item-id="address"]', "aria-label"))
        p.phone = re.sub(
            r"^Phone:\s*", "",
            attr_or_empty(page, 'button[data-item-id^="phone"]', "aria-label"))
        p.website = attr_or_empty(page, 'a[data-item-id="authority"]', "href")
        p.plus_code = re.sub(
            r"^Plus code:\s*", "",
            attr_or_empty(page, 'button[data-item-id="oloc"]', "aria-label"))

        p.website_type = classify_website(p.website)
        p.is_lead = "no" if p.website_type == "website" else "yes"

        # A ready-to-use WhatsApp link makes outreach one click away.
        if p.phone.startswith("+"):
            p.whatsapp_link = "https://wa.me/" + re.sub(r"\D", "", p.phone)

        # Hero photo of the place (useful in the dashboard).
        img = attr_or_empty(
            page, 'div[role="main"] button[jsaction*="heroHeaderImage"] img',
            "src")
        if img.startswith("http"):
            # Normalize Google image sizing suffix for a reasonable thumbnail.
            p.image_url = re.sub(r"=w\d+-h\d+[^\s]*$", "=w400-h300-k-no", img)

        # Opening hours: expand the hours section if present, read the table.
        try:
            hours_toggle = page.locator(
                'div[role="main"] [jsaction*="openhours"], '
                'div[role="main"] img[aria-label*="Hours"]'
            ).first
            if hours_toggle.count():
                hours_toggle.click(timeout=2000)
                page.wait_for_timeout(700)
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

    # ---- email extraction --------------------------------------------------

    def find_emails(self, website: str, timeout_ms: int = 15000) -> str:
        """Visit a business website and pull email addresses from the
        homepage and its contact page (if one is linked)."""
        if self.email_page is None:
            self.email_page = self.context.new_page()
        page = self.email_page
        found: list[str] = []

        def harvest(u: str) -> None:
            try:
                page.goto(u, wait_until="domcontentloaded", timeout=timeout_ms)
                page.wait_for_timeout(800)
                html = page.content()
            except Exception:
                return
            for m in EMAIL_RE.findall(html):
                e = m.strip().lower()
                if not EMAIL_JUNK.search(e) and e not in found:
                    found.append(e)

        harvest(website)
        try:
            contact = page.locator(
                'a[href*="contact" i], a:has-text("Contact")').first
            href = contact.get_attribute("href", timeout=1500) \
                if contact.count() else None
            if href:
                harvest(urllib.parse.urljoin(website, href))
        except Exception:
            pass

        return "; ".join(found[:5])


def run_scrape(
    query: str,
    max_results: int = 20,
    leads_only: bool = False,
    emails: bool = False,
    headless: bool = True,
    lang: str = "en",
    delay: float = 1.0,
    retries: int = 2,
    on_log: Optional[Callable[[str], None]] = None,
    on_place: Optional[Callable[[Place], None]] = None,
    on_total: Optional[Callable[[int], None]] = None,
    should_stop: Optional[Callable[[], bool]] = None,
) -> list[Place]:
    """Run a full scrape. Callbacks make this usable from the control center:

    - ``on_log(msg)``     called with progress messages
    - ``on_place(place)`` called as each place finishes (already filtered)
    - ``on_total(n)``     called once with the number of result links found
    - ``should_stop()``   polled between steps; return True to abort cleanly
    """
    log = on_log or (lambda m: print(f"[scraper] {m}", flush=True))
    stop = should_stop or (lambda: False)

    scraper = Scraper(headless=headless, lang=lang)
    places: list[Place] = []
    try:
        log(f"Searching for: {query!r}")
        urls = scraper.collect_result_urls(query, max_results, log, stop)
        log(f"Found {len(urls)} result link(s). Scraping details...")
        if on_total:
            on_total(len(urls))

        for i, url in enumerate(urls, 1):
            if stop():
                log("Stop requested — finishing up.")
                break
            place = None
            for attempt in range(1, retries + 1):
                try:
                    place = scraper.scrape_place(url)
                    break
                except Exception as e:
                    log(f"  [{i}/{len(urls)}] attempt {attempt} failed: "
                        f"{type(e).__name__}: {e}")
                    scraper.page.wait_for_timeout(2000)
            if place is None:
                continue
            if leads_only and place.is_lead != "yes":
                log(f"  [{i}/{len(urls)}] {place.name} — has website, skipped "
                    f"(leads-only)")
                time.sleep(delay)
                continue

            if emails and place.website and place.website_type == "website":
                place.emails = scraper.find_emails(place.website)

            places.append(place)
            if on_place:
                on_place(place)

            tag = "LEAD" if place.is_lead == "yes" else place.website_type
            log(f"  [{i}/{len(urls)}] {place.name or '(unnamed)'}  [{tag}]"
                + (f"  emails: {place.emails}" if place.emails else ""))
            time.sleep(delay)
    finally:
        scraper.close()

    leads = sum(1 for p in places if p.is_lead == "yes")
    social = sum(1 for p in places if p.website_type not in ("none", "website"))
    log(f"Summary: {len(places)} place(s) scraped — {leads} lead(s) without "
        f"a real website ({social} of them social-media-only).")
    return places


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Scrape Google Maps search results (no API key needed).")
    ap.add_argument("query", help='Search query, e.g. "restaurants in Beirut"')
    ap.add_argument("-n", "--max-results", type=int, default=20,
                    help="Maximum number of places to scrape (default 20)")
    ap.add_argument("-o", "--output", default="results",
                    help="Output file basename without extension "
                         "(default: results)")
    ap.add_argument("--format", choices=["csv", "json", "both"],
                    default="both", help="Output format (default: both)")
    ap.add_argument("--leads-only", action="store_true",
                    help="Keep only places WITHOUT a real website "
                         "(none or social-media-only)")
    ap.add_argument("--emails", action="store_true",
                    help="Visit each business website to extract email "
                         "addresses (slower)")
    ap.add_argument("--headful", action="store_true",
                    help="Run with a visible browser window")
    ap.add_argument("--lang", default="en", help="UI language (default: en)")
    ap.add_argument("--delay", type=float, default=1.0,
                    help="Delay in seconds between place visits (default 1.0)")
    ap.add_argument("--retries", type=int, default=2,
                    help="Attempts per place before skipping it (default 2)")
    args = ap.parse_args()

    inc_csv = IncrementalCSV(args.output + ".csv") \
        if args.format in ("csv", "both") else None

    places = run_scrape(
        query=args.query,
        max_results=args.max_results,
        leads_only=args.leads_only,
        emails=args.emails,
        headless=not args.headful,
        lang=args.lang,
        delay=args.delay,
        retries=args.retries,
        on_place=inc_csv.write if inc_csv else None,
    )
    if inc_csv:
        inc_csv.close()

    if not places:
        print("[scraper] No places scraped.", flush=True)
        return 1

    if args.format in ("json", "both"):
        with open(args.output + ".json", "w", encoding="utf-8") as f:
            json.dump([place_row(p) for p in places], f,
                      ensure_ascii=False, indent=2)
        print(f"[scraper] Wrote {args.output}.json", flush=True)
    if args.format in ("csv", "both"):
        print(f"[scraper] Wrote {args.output}.csv", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
