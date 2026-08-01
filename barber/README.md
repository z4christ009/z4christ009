# A7AA Barber Shop — website

A single-page site for the barber shop in Karm El Mehr. No build step, no
frameworks, no external requests — just open `index.html`.

Live at `/barber/` once the repo is published with GitHub Pages.

## The one thing you must edit

Open `index.html`, scroll to the bottom, and fill in the `SHOP` block:

```js
const SHOP = {
  whatsapp : "",   // e.g. "96170123456"  — country code first, digits only
  instagram: ""    // e.g. "https://instagram.com/a7aa.barber" — leave "" to hide the button
};
```

Until `whatsapp` is filled in, every WhatsApp button just scrolls down to the
booking form, and the form shows a reminder instead of opening a broken chat.

## Other things you'll probably want to change

Everything below is plain HTML in `index.html` — search for the comment shown.

| What | Where |
|---|---|
| Prices and service names | `<!-- EDIT ME: change the prices below -->` |
| Opening hours | `<!-- EDIT ME: change these to your real hours -->` |
| Address / map pin | the `Karm El Mehr, Lebanon` link in the **Hours & Location** section |
| The story text | the **About** section |
| Colours | the `:root` block at the top of the `<style>` |

Today's row in the opening hours is highlighted automatically — no need to
touch it.

## Photos

Photos live in `img/`. To swap one, drop in a new file with the same name and
keep it roughly the same shape:

- `hero.jpg` — 4:5 portrait
- `cut-01.jpg`, `cut-02.jpg` — 3:4 portrait
- `flyer.jpg` — square

Keep each one under ~250 KB so the site stays fast on mobile data.

## Fonts

Anton and Barlow are self-hosted in `fonts/` so the page loads the same on a
slow connection and doesn't depend on Google.
