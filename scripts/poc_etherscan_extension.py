#!/usr/bin/env python3
"""Public Etherscan L1/L2/L3 via VCU extension. No UI clicks."""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlparse

OUT = Path(os.environ.get("VCU_ETHERSCAN_OUT", str(Path.home() / "vcu-etherscan-labels")))
OUT.mkdir(parents=True, exist_ok=True)


def run(args, check=False):
    r = subprocess.run(args, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(f"cmd failed {args}: {r.stderr or r.stdout}")
    return r


def vcu_json(args):
    r = run(["vcu", *args])
    try:
        return json.loads(r.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": r.stdout, "err": r.stderr}


def main():
    health = json.loads(
        subprocess.check_output(["curl", "-fsS", "http://127.0.0.1:17890/v1/health"], text=True)
    )
    if not (health.get("data") or {}).get("extension_polling"):
        print("extension_polling is false; run bash scripts/start_agent_edge.sh", file=sys.stderr)
        return 1
    started = vcu_json(["session", "start", "--backend", "extension", "--browser", "edge", "--json"])
    (OUT / "ext_session.json").write_text(json.dumps(started, indent=2))
    sid = (started.get("data") or {}).get("session_id")
    if not sid:
        print(started, file=sys.stderr)
        return 1
    print("SID", sid, flush=True)

    def nav(url):
        print("NAV", url, flush=True)
        vcu_json(["navigate", "--session", sid, "--url", url, "--json"])
        vcu_json(["wait", "--session", sid, "--ms", "3000", "--json"])

    def extract(sel):
        return vcu_json(["extract", "--session", sid, "--selector", sel, "--json"])

    nav("https://etherscan.io/labelcloud")
    l1 = extract('a[href*="/label/"]')
    (OUT / "l1_hrefs.json").write_text(json.dumps(l1, indent=2))
    matches = ((l1.get("data") or {}).get("matches")) or []
    hrefs = []
    seen = set()
    for m in matches:
        href = m.get("href") or ""
        if "/label/" in href and href not in seen:
            seen.add(href)
            hrefs.append(href)
    slugs = []
    for h in hrefs:
        path = urlparse(h).path
        m = re.search(r"/(accounts|tokens)/label/([^/?#]+)", path)
        if m:
            slugs.append({"kind": m.group(1), "slug": m.group(2), "href": h})
            continue
        m = re.search(r"/label/([^/?#]+)", path)
        if m:
            slugs.append({"kind": "unknown", "slug": m.group(1), "href": h})
    (OUT / "l1_slugs.json").write_text(
        json.dumps({"count": len(hrefs), "hrefs": hrefs[:300], "slugs": slugs[:300]}, indent=2)
    )
    print("l1_hrefs", len(hrefs), "slugs", len(slugs), flush=True)

    prefer = [
        "aave", "uniswap", "binance", "tether", "usd-coin", "1inch", "okx", "coinbase",
        "kraken", "weth", "usdt", "circle", "bitfinex", "lido", "maker", "compound",
        "opensea", "blur", "ens", "sushiswap",
    ]
    by = {}
    for s in slugs:
        by.setdefault(s["slug"].lower(), []).append(s)
    plan = []
    have = set()
    for p in prefer:
        for s in by.get(p, []):
            key = (s["kind"], s["slug"])
            if s["kind"] in ("accounts", "tokens") and key not in have:
                plan.append(s)
                have.add(key)
    for s in slugs:
        key = (s["kind"], s["slug"])
        if s["kind"] in ("accounts", "tokens") and key not in have:
            plan.append(s)
            have.add(key)
        if len(plan) >= 10:
            break
    plan = plan[:10]
    (OUT / "l3_plan.json").write_text(json.dumps(plan, indent=2))

    results = []
    for item in plan:
        url = item.get("href") or f"https://etherscan.io/{item['kind']}/label/{item['slug']}"
        nav(url)
        body = extract('a[href*="/address/"]')
        dest = OUT / f"l3_{item['kind']}_{item['slug']}_extract.json"
        dest.write_text(json.dumps(body, indent=2))
        if not body.get("ok"):
            print("  extract_fail", (body.get("error") or body) , flush=True)
        matches = ((body.get("data") or {}).get("matches")) or []
        wall_body = extract("h1,h2,button")
        wall_matches = ((wall_body.get("data") or {}).get("matches")) or []
        blob = " ".join((m.get("text") or "") for m in (matches + wall_matches))
        wall = bool(re.search(r"Sign In for Continued Access|Log In", blob, re.I))
        addrs = []
        seen_a = set()
        for m in matches:
            href = m.get("href") or ""
            text = (m.get("text") or "").strip()
            mm = re.search(r"0x[a-fA-F0-9]{40}", href) or re.search(r"0x[a-fA-F0-9]{40}", text)
            if not mm:
                continue
            addr = mm.group(0)
            if addr in seen_a:
                continue
            seen_a.add(addr)
            addrs.append({"address": addr, "name_tag": text[:120], "href": href or None})
        rec = {
            "kind": item["kind"],
            "slug": item["slug"],
            "url": url,
            "login_wall": wall,
            "count_sample": len(addrs),
            "addresses": addrs[:20],
        }
        results.append(rec)
        print("  sample", rec["count_sample"], "wall", wall, flush=True)
    (OUT / "l3_batch.json").write_text(json.dumps(results, indent=2))

    tree = {}
    if (OUT / "labels_tree.json").exists():
        try:
            tree = json.loads((OUT / "labels_tree.json").read_text())
        except Exception:
            tree = {}
    extra = tree.get("l3_extra") or {}
    for item in results:
        extra[f"{item['slug']}_{item['kind']}"] = item
    tree.update({
        "mode": "EXTENSION_AGENT_WINDOW",
        "login_full_list": False,
        "login_wall_after_first_page": any(x.get("login_wall") for x in results),
        "l1": {
            "source": "https://etherscan.io/labelcloud",
            "href_sample_count": len(hrefs),
            "href_sample": hrefs[:30],
            "slug_count": len(slugs),
        },
        "l3_extra": extra,
        "summary": {
            "l1_href_count": len(hrefs),
            "l3_public_pages": len(results),
            "l3_public_first_page_pattern": "about 7 addresses then Sign In for Continued Access",
            "samples": list(extra)[:12],
        },
    })
    (OUT / "labels_tree.json").write_text(json.dumps(tree, indent=2))
    run(["vcu", "session", "stop", sid, "--json"])
    print("ETHERSCAN_EXT_DONE", OUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
