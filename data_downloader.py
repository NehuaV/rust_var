"""Download price history for a list of tickers into data/.

Each file is saved as  data/{period}_{ticker}_{date}.csv  with a two-column
format (Date, Close) that matches what the Rust loader expects.

A SHA-256 hash of every file is stored in data/hashes.json.  A ticker is
skipped if its CSV already exists and its hash matches the stored value for
today's date, avoiding unnecessary API calls on the same day.  The cache is
automatically invalidated the next calendar day.  Pass --force to bypass the
cache regardless.

Usage
-----
    uv run data_downloader.py                      # defaults below
    uv run data_downloader.py AAPL MSFT            # override tickers from CLI
    uv run data_downloader.py --force AAPL MSFT    # ignore cache, re-download
"""

import datetime
import hashlib
import json
import pathlib
import sys

import pandas as pd
import yfinance as yf

# ── Configuration ──────────────────────────────────────────────────────────────

# List of publicly listed companies' tickers (US large caps, tech, industry leaders, mega/large caps, diversity of sectors)
DEFAULT_TICKERS = [
    "AAPL",  # Apple Inc.
    "MSFT",  # Microsoft Corp
    "GOOG",  # Alphabet Inc. Class C
    "GOOGL",  # Alphabet Inc. Class A
    "META",  # Meta Platforms (Facebook)
    "AMZN",  # Amazon.com
    "TSLA",  # Tesla Inc.
    "NVDA",  # NVIDIA Corp
    "BRK-B",  # Berkshire Hathaway (Class B)
    "JPM",  # JPMorgan Chase
    "JNJ",  # Johnson & Johnson
    "V",  # Visa Inc.
    "WMT",  # Walmart Inc.
    "PG",  # Procter & Gamble
    "MA",  # Mastercard
    "DIS",  # Walt Disney Co.
    "HD",  # Home Depot
    "BAC",  # Bank of America
    "XOM",  # ExxonMobil
    "CVX",  # Chevron Corp
    "KO",  # Coca-Cola
    "CSCO",  # Cisco Systems
    "PEP",  # PepsiCo
    "MCD",  # McDonald's
    "INTC",  # Intel Corp
    "ORCL",  # Oracle Corp
    "IBM",  # IBM Corp
    "PLTR",  # Palantir Technologies
    "ABBV",  # AbbVie Inc.
    "NFLX",  # Netflix Inc.
    "MRK",  # Merck & Co.
    "PFE",  # Pfizer Inc.
    "T",  # AT&T Inc.
    "UNH",  # UnitedHealth Group
    "COST",  # Costco Wholesale
    "ABT",  # Abbott Labs
    "NKE",  # NIKE Inc.
    "MDT",  # Medtronic plc
    "LLY",  # Eli Lilly
    "CRM",  # Salesforce, Inc.
    "LIN",  # Linde plc
    "ACN",  # Accenture plc
    "LOW",  # Lowe’s Companies
    "PM",  # Philip Morris Intl
    "DHR",  # Danaher Corp
    "TMO",  # Thermo Fisher Scientific
    "NEE",  # NextEra Energy
    "HON",  # Honeywell International
    "TXN",  # Texas Instruments
    "BMY",  # Bristol Myers Squibb
    "C",  # Citigroup Inc.
    "UPS",  # United Parcel Service
    "BA",  # Boeing Co.
    "GE",  # General Electric
    "SBUX",  # Starbucks
    "AMAT",  # Applied Materials
    "UNP",  # Union Pacific
    "SPGI",  # S&P Global Inc.
    "MS",  # Morgan Stanley
    "BLK",  # BlackRock Inc.
    "QCOM",  # Qualcomm
    "GS",  # Goldman Sachs
    "CAT",  # Caterpillar Inc.
    "SCHW",  # Charles Schwab
    "FDX",  # FedEx
    "MMM",  # 3M Company
    "MDLZ",  # Mondelez International
    "GILD",  # Gilead Sciences
    "LMT",  # Lockheed Martin
    "DE",  # Deere & Company
    "BKNG",  # Booking Holdings
    "NOW",  # ServiceNow, Inc.
    "ISRG",  # Intuitive Surgical
    "ADP",  # Automatic Data Processing
    "EL",  # Estee Lauder Companies
    "CB",  # Chubb Limited
    "SYK",  # Stryker Corp
    "MO",  # Altria Group
    "CI",  # Cigna Corp
    "ZTS",  # Zoetis Inc.
    "VRTX",  # Vertex Pharmaceuticals
    "GM",  # General Motors
    "VRTX",  # Vertex Pharmaceuticals
    "CL",  # Colgate-Palmolive
    "SO",  # Southern Company
    "DUK",  # Duke Energy
    "REGN",  # Regeneron Pharmaceuticals
    "AON",  # Aon plc
    "USB",  # U.S. Bancorp
    "PGR",  # Progressive Corp
    "TGT",  # Target Corp
    "AXP",  # American Express
    "FIS",  # Fidelity National Info
]
PERIOD = "1y"
DATE = datetime.date.today().isoformat()   # YYYY-MM-DD — cache key expires daily
DATA_DIR = pathlib.Path(__file__).parent / "data"
HASHES_FILE = DATA_DIR / "hashes.json"

# ──────────────────────────────────────────────────────────────────────────────


def _cache_key(ticker: str, period: str, date: str = DATE) -> str:
    return f"{period}_{ticker}_{date}"


def _csv_path(ticker: str, period: str, date: str = DATE) -> pathlib.Path:
    return DATA_DIR / f"{_cache_key(ticker, period, date)}.csv"


def _sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def _load_hashes() -> dict:
    if HASHES_FILE.exists():
        return json.loads(HASHES_FILE.read_text())
    return {}


def _save_hashes(hashes: dict) -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    HASHES_FILE.write_text(json.dumps(hashes, indent=2))


def _fetch(ticker: str, period: str) -> pd.Series:
    raw = yf.download(ticker, period=period, auto_adjust=True, progress=False)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.squeeze()
    raw.name = ticker
    return raw


def ensure_ticker(
    ticker: str, period: str, hashes: dict, force: bool
) -> tuple[bool, str]:
    """Download ticker if not already cached. Returns (success, status_message)."""
    key = _cache_key(ticker, period)
    csv = _csv_path(ticker, period)

    if not force and csv.exists() and key in hashes and _sha256(csv) == hashes[key]:
        return True, "cached — skipped"

    try:
        prices = _fetch(ticker, period)
        if prices.empty:
            return False, "no data returned from Yahoo Finance"
    except Exception as exc:
        return False, f"download failed: {exc}"

    csv.write_text(prices.to_csv())
    hashes[key] = _sha256(csv)
    _save_hashes(hashes)
    return True, f"{len(prices)} rows  →  {csv}"


def main(tickers: list[str], period: str, force: bool) -> None:
    print(
        f"Downloading {len(tickers)} ticker(s), period={period}"
        + (" [force]\n" if force else "\n")
    )
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    hashes = _load_hashes()
    ok, failed = [], []

    for ticker in tickers:
        success, msg = ensure_ticker(ticker, period, hashes, force)
        status = "✓" if success else "✗"
        print(f"  {status} [{ticker}] {msg}")
        (ok if success else failed).append(ticker)

    print(f"\nDone — {len(ok)} ok, {len(failed)} failed.")
    if failed:
        print(f"Failed tickers: {', '.join(failed)}")
        sys.exit(1)


if __name__ == "__main__":
    args = sys.argv[1:]
    force = "--force" in args
    tickers = [a for a in args if not a.startswith("--")] or DEFAULT_TICKERS
    main(tickers, PERIOD, force)
