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

# Business types that genuinely benefit from having a website: customers
# check menus, book appointments, browse portfolios/catalogs, or compare
# offerings online before choosing. Used by the "Website Prospects" campaign
# mode, which scans all of these in an area and keeps only the ones without
# a real website.
WEBSITE_PROSPECT_TYPES = [
    "restaurants", "cafes", "catering services", "pastry shops",
    "hotels", "guesthouses", "event venues", "wedding venues",
    "beauty salons", "barber shops", "spas", "gyms",
    "dental clinics", "medical clinics", "physiotherapy clinics",
    "photographers", "interior designers", "architects",
    "real estate agencies", "travel agencies", "car rental agencies",
    "law firms", "accounting firms", "private schools", "nurseries",
]

# Speed profiles. "block_heavy" aborts image/media/font requests (the map
# tiles and photos Google loads are useless to us — the photo URL is read
# from the DOM attribute, which works even when the download is blocked).
SPEED_PROFILES = {
    "fast":     {"delay": 0.25, "settle_ms": 350,  "search_settle_ms": 1500,
                 "hours": False, "retries": 1, "block_heavy": True},
    "balanced": {"delay": 0.8,  "settle_ms": 1000, "search_settle_ms": 2500,
                 "hours": True,  "retries": 2, "block_heavy": True},
    "thorough": {"delay": 1.5,  "settle_ms": 1500, "search_settle_ms": 3000,
                 "hours": True,  "retries": 3, "block_heavy": False},
}

FIELDNAMES = [
    "name", "category", "status", "rating", "reviews_count", "price_level",
    "address", "phone", "whatsapp_link", "website", "website_type", "is_lead",
    "lead_score", "emails", "plus_code", "latitude", "longitude",
    "opening_hours", "image_url", "google_maps_url",
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
    lead_score: int = 0          # 0-100, how promising this lead is
    emails: str = ""
    plus_code: str = ""
    latitude: str = ""
    longitude: str = ""
    opening_hours: dict = field(default_factory=dict)
    image_url: str = ""
    google_maps_url: str = ""


def score_lead(p: "Place") -> int:
    """0-100: how promising is this lead for selling a website?

    Rewards established businesses (reviews prove real customers), quality
    (rating — a good business is worth working with and can afford it),
    reachability (phone), and buying intent (they already invest in an
    Instagram/Facebook page or a hosted menu but own no real site).
    """
    if p.is_lead != "yes" or p.status != "Operational":
        return 0
    score = 0

    try:
        reviews = int(p.reviews_count or 0)
    except ValueError:
        reviews = 0
    for threshold, pts in ((200, 40), (100, 35), (50, 30), (20, 22),
                           (10, 15), (5, 8)):
        if reviews >= threshold:
            score += pts
            break
    else:
        score += 3

    try:
        rating = float(p.rating or 0)
    except ValueError:
        rating = 0
    if rating >= 4.5:
        score += 20
    elif rating >= 4.0:
        score += 15
    elif rating >= 3.5:
        score += 8
    else:
        score += 3

    if p.phone:
        score += 15

    if p.website_type in ("hosted-page", "shortlink"):
        score += 20   # they already paid/tried for some web presence
    elif p.website_type in ("instagram", "facebook", "whatsapp",
                            "linktree", "tiktok", "social"):
        score += 15   # active online, easy conversation
    else:
        score += 10   # nothing at all — biggest need

    return min(score, 100)


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

    def __init__(self, headless: bool = True, lang: str = "en",
                 speed: str = "balanced"):
        self.profile = SPEED_PROFILES.get(speed, SPEED_PROFILES["balanced"])
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
        if self.profile["block_heavy"]:
            self.context.route(
                "**/*",
                lambda route: route.abort()
                if route.request.resource_type in ("image", "media", "font")
                else route.continue_(),
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
                            should_stop: Callable[[], bool],
                            seen: Optional[set] = None) -> list[str]:
        """``seen`` lets campaign runs dedupe places across queries (a cafe
        can appear under both "cafes" and "restaurants")."""
        page = self.page
        url = "https://www.google.com/maps/search/" + urllib.parse.quote(query)
        page.goto(url, wait_until="domcontentloaded", timeout=60000)
        accept_consent(page)

        # A direct hit on a single place redirects to /maps/place/, no feed.
        page.wait_for_timeout(self.profile["search_settle_ms"])
        if "/maps/place/" in page.url:
            return [page.url]

        try:
            page.wait_for_selector(FEED_SELECTOR, timeout=15000)
        except PWTimeout:
            log("No results feed found; the query may have returned nothing.")
            return []

        urls: list[str] = []
        if seen is None:
            seen = set()
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
            page.wait_for_timeout(
                1000 if self.profile["block_heavy"] else 1600)

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
        page.wait_for_timeout(self.profile["settle_ms"])

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

        p.lead_score = score_lead(p)

        # Hero photo of the place (useful in the dashboard). The <img> src is
        # only populated once images load, so in fast mode (image downloads
        # blocked) fall back to the photo URL Google embeds in the page data.
        img = attr_or_empty(
            page, 'div[role="main"] button[jsaction*="heroHeaderImage"] img',
            "src")
        if not img.startswith("http"):
            m = re.search(
                r'https://lh\d+\.googleusercontent\.com/'
                r'(?:p|gps-cs-s)/[A-Za-z0-9_-]{10,}',
                page.content())
            img = m.group(0) if m else ""
        if img.startswith("http") and "staticmap" not in img:
            # Normalize Google image sizing suffix for a reasonable thumbnail.
            p.image_url = re.sub(r"=w\d+-h\d+[^\s]*$", "=w400-h300-k-no", img)

        # Opening hours: expand the hours section if present, read the table.
        # Skipped entirely in fast mode.
        if not self.profile["hours"]:
            return p
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
    query: str = "",
    max_results: int = 20,
    leads_only: bool = False,
    emails: bool = False,
    headless: bool = True,
    lang: str = "en",
    speed: str = "balanced",
    delay: Optional[float] = None,
    retries: Optional[int] = None,
    queries: Optional[list[str]] = None,
    on_log: Optional[Callable[[str], None]] = None,
    on_place: Optional[Callable[[Place], None]] = None,
    on_total: Optional[Callable[[int], None]] = None,
    should_stop: Optional[Callable[[], bool]] = None,
) -> list[Place]:
    """Run a full scrape. Callbacks make this usable from the control center:

    - ``on_log(msg)``     called with progress messages
    - ``on_place(place)`` called as each place finishes (already filtered)
    - ``on_total(n)``     called once with the expected total (for progress)
    - ``should_stop()``   polled between steps; return True to abort cleanly

    ``speed`` is one of "fast", "balanced", "thorough" (see SPEED_PROFILES).
    ``delay``/``retries`` override the profile when given explicitly.

    Pass ``queries`` (a list) instead of ``query`` to run a multi-query
    campaign in one browser session: places are deduplicated across queries
    and the run stops once ``max_results`` places have been *kept* (after
    the leads-only filter), which makes "give me 30 leads" work across
    many business types.
    """
    log = on_log or (lambda m: print(f"[scraper] {m}", flush=True))
    stop = should_stop or (lambda: False)

    profile = SPEED_PROFILES.get(speed, SPEED_PROFILES["balanced"])
    delay = profile["delay"] if delay is None else delay
    retries = profile["retries"] if retries is None else retries

    campaign = queries is not None
    query_list = queries if campaign else [query]
    # In a campaign, max_results caps *kept* places; per-query URL collection
    # is capped so no single type dominates the run.
    per_query_cap = max_results if not campaign else \
        max(4, -(-max_results * 2 // len(query_list)))

    started = time.time()
    scraper = Scraper(headless=headless, lang=lang, speed=speed)
    places: list[Place] = []
    seen_ids: set = set()
    try:
        if on_total:
            on_total(max_results if campaign else 0)
        for qi, q in enumerate(query_list, 1):
            if stop() or len(places) >= max_results:
                break
            prefix = f"[{qi}/{len(query_list)}] " if campaign else ""
            log(f"{prefix}Searching for: {q!r}  (speed: {speed})")
            urls = scraper.collect_result_urls(
                q, min(per_query_cap, max_results - len(places))
                if campaign else max_results,
                log, stop, seen=seen_ids)
            log(f"{prefix}Found {len(urls)} new result link(s).")
            if not campaign and on_total:
                on_total(len(urls))

            for i, url in enumerate(urls, 1):
                if stop():
                    log("Stop requested — finishing up.")
                    break
                if len(places) >= max_results:
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
                    log(f"  [{i}/{len(urls)}] {place.name} — has website, "
                        f"skipped (leads-only)")
                    time.sleep(delay)
                    continue

                if emails and place.website and \
                        place.website_type == "website":
                    place.emails = scraper.find_emails(place.website)

                places.append(place)
                if on_place:
                    on_place(place)

                tag = "LEAD" if place.is_lead == "yes" else place.website_type
                score = f" score {place.lead_score}" \
                    if place.is_lead == "yes" else ""
                log(f"  [{i}/{len(urls)}] {place.name or '(unnamed)'}  "
                    f"[{tag}]{score}"
                    + (f"  emails: {place.emails}" if place.emails else ""))
                time.sleep(delay)
    finally:
        scraper.close()

    leads = sum(1 for p in places if p.is_lead == "yes")
    social = sum(1 for p in places if p.website_type not in ("none", "website"))
    elapsed = time.time() - started
    rate = (60 * len(places) / elapsed) if elapsed and places else 0
    log(f"Summary: {len(places)} place(s) scraped in {elapsed:.0f}s "
        f"({rate:.1f}/min) — {leads} lead(s) without a real website "
        f"({social} of them social-media-only).")
    return places


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Scrape Google Maps search results (no API key needed).")
    ap.add_argument("query", nargs="?", default="",
                    help='Search query, e.g. "restaurants in Beirut"')
    ap.add_argument("--prospects", metavar="AREA", default="",
                    help="Website-prospects campaign: scan every business "
                         "type that needs a website in AREA, keeping only "
                         "places without a real site "
                         '(e.g. --prospects "Batroun")')
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
    ap.add_argument("--speed", choices=list(SPEED_PROFILES),
                    default="balanced",
                    help="fast = ~2-3x quicker, skips opening hours; "
                         "balanced = everything, still quick (default); "
                         "thorough = slowest, gentlest on Google")
    ap.add_argument("--headful", action="store_true",
                    help="Run with a visible browser window")
    ap.add_argument("--lang", default="en", help="UI language (default: en)")
    ap.add_argument("--delay", type=float, default=None,
                    help="Override the speed profile's delay between places")
    ap.add_argument("--retries", type=int, default=None,
                    help="Override the speed profile's attempts per place")
    args = ap.parse_args()

    if not args.query and not args.prospects:
        ap.error("provide a query or --prospects AREA")

    queries = None
    leads_only = args.leads_only
    if args.prospects:
        area = args.prospects.strip()
        suffix = "" if re.search(r"lebanon", area, re.I) else ", Lebanon"
        queries = [f"{t} in {area}{suffix}" for t in WEBSITE_PROSPECT_TYPES]
        leads_only = True

    inc_csv = IncrementalCSV(args.output + ".csv") \
        if args.format in ("csv", "both") else None

    places = run_scrape(
        query=args.query,
        queries=queries,
        max_results=args.max_results,
        leads_only=leads_only,
        emails=args.emails,
        headless=not args.headful,
        lang=args.lang,
        speed=args.speed,
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
