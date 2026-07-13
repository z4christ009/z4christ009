#!/usr/bin/env python3
"""Control center for the Google Maps lead scraper.

A local web dashboard where you can start scrape jobs, watch live progress,
browse/filter the leads, and download CSV/JSON — no command line needed.

Run:  python app.py          (or use start.sh / start.bat)
Then open http://localhost:8765
"""

from __future__ import annotations

import json
import os
import threading
import time
import uuid
import webbrowser

from flask import Flask, jsonify, request, send_file, render_template, abort

import scraper as engine

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
JOBS_DIR = os.path.join(BASE_DIR, "data", "jobs")
os.makedirs(JOBS_DIR, exist_ok=True)

PORT = 8765

app = Flask(__name__)

# In-memory job registry. Finished jobs are also persisted to disk so the
# dashboard survives restarts.
JOBS: dict[str, dict] = {}
JOBS_LOCK = threading.Lock()


def job_dir(job_id: str) -> str:
    return os.path.join(JOBS_DIR, job_id)


def public_meta(job: dict) -> dict:
    return {k: job.get(k, "") for k in (
        "id", "query", "max_results", "leads_only", "emails", "speed",
        "status", "created_at", "total_urls", "scraped", "leads", "error",
    )}


def save_job(job: dict) -> None:
    d = job_dir(job["id"])
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "meta.json"), "w", encoding="utf-8") as f:
        json.dump(public_meta(job), f, ensure_ascii=False, indent=2)
    with open(os.path.join(d, "results.json"), "w", encoding="utf-8") as f:
        json.dump(job["results"], f, ensure_ascii=False, indent=2)
    with open(os.path.join(d, "log.txt"), "w", encoding="utf-8") as f:
        f.write("\n".join(job["log"]))


def load_saved_jobs() -> None:
    if not os.path.isdir(JOBS_DIR):
        return
    for job_id in os.listdir(JOBS_DIR):
        meta_path = os.path.join(JOBS_DIR, job_id, "meta.json")
        if not os.path.isfile(meta_path):
            continue
        try:
            with open(meta_path, encoding="utf-8") as f:
                meta = json.load(f)
            results_path = os.path.join(JOBS_DIR, job_id, "results.json")
            results = []
            if os.path.isfile(results_path):
                with open(results_path, encoding="utf-8") as f:
                    results = json.load(f)
            log_path = os.path.join(JOBS_DIR, job_id, "log.txt")
            log_lines = []
            if os.path.isfile(log_path):
                with open(log_path, encoding="utf-8") as f:
                    log_lines = f.read().splitlines()
            # A job that was "running" when the server died is effectively
            # stopped.
            if meta.get("status") in ("running", "queued"):
                meta["status"] = "stopped"
            job = {**meta, "results": results, "log": log_lines,
                   "stop_event": threading.Event()}
            JOBS[job_id] = job
        except Exception:
            continue


def run_job(job: dict) -> None:
    job["status"] = "running"
    stop_event: threading.Event = job["stop_event"]
    csv_path = os.path.join(job_dir(job["id"]), "results.csv")
    os.makedirs(job_dir(job["id"]), exist_ok=True)
    inc_csv = engine.IncrementalCSV(csv_path)

    def on_log(msg: str) -> None:
        job["log"].append(msg)

    def on_place(place: engine.Place) -> None:
        row = engine.place_row(place)
        job["results"].append(row)
        job["scraped"] = len(job["results"])
        job["leads"] = sum(1 for r in job["results"] if r["is_lead"] == "yes")
        inc_csv.write(place)

    def on_total(n: int) -> None:
        job["total_urls"] = n

    try:
        engine.run_scrape(
            query=job["query"],
            max_results=job["max_results"],
            leads_only=job["leads_only"],
            emails=job["emails"],
            headless=True,
            speed=job.get("speed", "balanced"),
            on_log=on_log,
            on_place=on_place,
            on_total=on_total,
            should_stop=stop_event.is_set,
        )
        job["status"] = "stopped" if stop_event.is_set() else "done"
    except Exception as e:
        job["status"] = "error"
        job["error"] = f"{type(e).__name__}: {e}"
        job["log"].append(f"ERROR: {job['error']}")
    finally:
        inc_csv.close()
        save_job(job)


@app.route("/")
def index():
    return render_template("index.html")


@app.route("/api/jobs", methods=["GET"])
def list_jobs():
    with JOBS_LOCK:
        jobs = sorted(JOBS.values(), key=lambda j: j["created_at"],
                      reverse=True)
        return jsonify([public_meta(j) for j in jobs])


@app.route("/api/jobs", methods=["POST"])
def create_job():
    data = request.get_json(force=True)
    query = (data.get("query") or "").strip()
    if not query:
        return jsonify({"error": "query is required"}), 400
    try:
        max_results = max(1, min(int(data.get("max_results") or 20), 200))
    except (TypeError, ValueError):
        max_results = 20

    speed = data.get("speed") or "balanced"
    if speed not in engine.SPEED_PROFILES:
        speed = "balanced"

    job = {
        "id": uuid.uuid4().hex[:12],
        "query": query,
        "max_results": max_results,
        "leads_only": bool(data.get("leads_only")),
        "emails": bool(data.get("emails")),
        "speed": speed,
        "status": "queued",
        "created_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "total_urls": 0,
        "scraped": 0,
        "leads": 0,
        "error": "",
        "results": [],
        "log": [],
        "stop_event": threading.Event(),
    }
    with JOBS_LOCK:
        JOBS[job["id"]] = job
    threading.Thread(target=run_job, args=(job,), daemon=True).start()
    return jsonify(public_meta(job)), 201


@app.route("/api/jobs/<job_id>", methods=["GET"])
def get_job(job_id: str):
    job = JOBS.get(job_id)
    if not job:
        abort(404)
    after = request.args.get("after", type=int, default=0)
    return jsonify({
        **public_meta(job),
        "log": job["log"][after:],
        "log_offset": len(job["log"]),
        "results": job["results"],
    })


@app.route("/api/jobs/<job_id>/stop", methods=["POST"])
def stop_job(job_id: str):
    job = JOBS.get(job_id)
    if not job:
        abort(404)
    job["stop_event"].set()
    return jsonify({"ok": True})


@app.route("/api/jobs/<job_id>/delete", methods=["POST"])
def delete_job(job_id: str):
    job = JOBS.get(job_id)
    if not job:
        abort(404)
    if job["status"] == "running":
        return jsonify({"error": "stop the job first"}), 400
    with JOBS_LOCK:
        JOBS.pop(job_id, None)
    d = job_dir(job_id)
    if os.path.isdir(d):
        for name in os.listdir(d):
            try:
                os.remove(os.path.join(d, name))
            except OSError:
                pass
        try:
            os.rmdir(d)
        except OSError:
            pass
    return jsonify({"ok": True})


@app.route("/download/<job_id>.<fmt>")
def download(job_id: str, fmt: str):
    job = JOBS.get(job_id)
    if not job or fmt not in ("csv", "json"):
        abort(404)
    safe_query = "".join(
        c if c.isalnum() else "_" for c in job["query"])[:40].strip("_")
    if fmt == "csv":
        path = os.path.join(job_dir(job_id), "results.csv")
        if not os.path.isfile(path):
            abort(404)
        return send_file(path, as_attachment=True,
                         download_name=f"{safe_query}.csv")
    # JSON straight from memory so running jobs can be downloaded too.
    payload = json.dumps(job["results"], ensure_ascii=False, indent=2)
    path = os.path.join(job_dir(job_id), "results_download.json")
    os.makedirs(job_dir(job_id), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(payload)
    return send_file(path, as_attachment=True,
                     download_name=f"{safe_query}.json")


def open_browser_later() -> None:
    time.sleep(1.2)
    try:
        webbrowser.open(f"http://localhost:{PORT}")
    except Exception:
        pass


if __name__ == "__main__":
    load_saved_jobs()
    threading.Thread(target=open_browser_later, daemon=True).start()
    print(f"\n  Control center running at:  http://localhost:{PORT}\n")
    app.run(host="127.0.0.1", port=PORT, debug=False, threaded=True)
