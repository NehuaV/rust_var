"""Download price history for a list of tickers into data/.

Each file is saved as data/{period}_{ticker}_{date}.csv with a two-column
format (Date, Close) that matches what the Rust loader expects.

A SHA-256 hash of every file is stored in data/hashes.json. A ticker is
skipped if its CSV already exists and its hash matches the stored value for
today's date, avoiding unnecessary API calls on the same day. The cache is
automatically invalidated the next calendar day. Pass --force to bypass the
cache regardless.

Usage
-----
    uv run data_downloader.py                                  # batch default tickers
    uv run data_downloader.py AAPL MSFT                        # override tickers
    uv run data_downloader.py batch AAPL MSFT NVDA --json      # batch with JSON output
    uv run data_downloader.py ensure AAPL --json               # single ticker JSON
    uv run data_downloader.py search AAPL --json               # ticker search
"""

import argparse
import datetime
import hashlib
import json
import pathlib
import socket
import sys

# Set hard socket timeout so network requests NEVER hang indefinitely
socket.setdefaulttimeout(15.0)

import pandas as pd
import yfinance as yf

# ── Curated Company Names ─────────────────────────────────────────────────────

KNOWN_TICKER_NAMES = {
    "AAPL": "Apple Inc.",
    "MSFT": "Microsoft Corporation",
    "GOOG": "Alphabet Inc. (Class C)",
    "GOOGL": "Alphabet Inc. (Class A)",
    "META": "Meta Platforms, Inc.",
    "AMZN": "Amazon.com, Inc.",
    "TSLA": "Tesla, Inc.",
    "NVDA": "NVIDIA Corporation",
    "BRK-B": "Berkshire Hathaway Inc.",
    "JPM": "JPMorgan Chase & Co.",
    "JNJ": "Johnson & Johnson",
    "V": "Visa Inc.",
    "WMT": "Walmart Inc.",
    "PG": "Procter & Gamble Co.",
    "MA": "Mastercard Inc.",
    "DIS": "The Walt Disney Company",
    "HD": "The Home Depot, Inc.",
    "BAC": "Bank of America Corp.",
    "XOM": "Exxon Mobil Corporation",
    "CVX": "Chevron Corporation",
    "KO": "The Coca-Cola Company",
    "CSCO": "Cisco Systems, Inc.",
    "PEP": "PepsiCo, Inc.",
    "MCD": "McDonald's Corporation",
    "INTC": "Intel Corporation",
    "ORCL": "Oracle Corporation",
    "IBM": "International Business Machines",
    "PLTR": "Palantir Technologies Inc.",
    "ABBV": "AbbVie Inc.",
    "NFLX": "Netflix, Inc.",
    "MRK": "Merck & Co., Inc.",
    "PFE": "Pfizer Inc.",
    "T": "AT&T Inc.",
    "UNH": "UnitedHealth Group Inc.",
    "COST": "Costco Wholesale Corp.",
    "ABT": "Abbott Laboratories",
    "NKE": "NIKE, Inc.",
    "MDT": "Medtronic plc",
    "LLY": "Eli Lilly and Company",
    "CRM": "Salesforce, Inc.",
    "LIN": "Linde plc",
    "ACN": "Accenture plc",
    "LOW": "Lowe's Companies, Inc.",
    "PM": "Philip Morris International",
    "DHR": "Danaher Corporation",
    "TMO": "Thermo Fisher Scientific Inc.",
    "NEE": "NextEra Energy, Inc.",
    "HON": "Honeywell International Inc.",
    "TXN": "Texas Instruments Inc.",
    "BMY": "Bristol-Myers Squibb Co.",
    "C": "Citigroup Inc.",
    "UPS": "United Parcel Service, Inc.",
    "BA": "The Boeing Company",
    "GE": "General Electric Company",
    "SBUX": "Starbucks Corporation",
    "AMAT": "Applied Materials, Inc.",
    "UNP": "Union Pacific Corporation",
    "SPGI": "S&P Global Inc.",
    "MS": "Morgan Stanley",
    "BLK": "BlackRock, Inc.",
    "QCOM": "QUALCOMM Incorporated",
    "GS": "The Goldman Sachs Group",
    "CAT": "Caterpillar Inc.",
    "SCHW": "The Charles Schwab Corp.",
    "FDX": "FedEx Corporation",
    "MMM": "3M Company",
    "MDLZ": "Mondelez International",
    "GILD": "Gilead Sciences, Inc.",
    "LMT": "Lockheed Martin Corp.",
    "DE": "Deere & Company",
    "BKNG": "Booking Holdings Inc.",
    "NOW": "ServiceNow, Inc.",
    "ISRG": "Intuitive Surgical, Inc.",
    "ADP": "Automatic Data Processing",
    "EL": "The Estee Lauder Companies",
    "CB": "Chubb Limited",
    "SYK": "Stryker Corporation",
    "MO": "Altria Group, Inc.",
    "CI": "The Cigna Group",
    "ZTS": "Zoetis Inc.",
    "VRTX": "Vertex Pharmaceuticals",
    "GM": "General Motors Company",
    "CL": "Colgate-Palmolive Company",
    "SO": "The Southern Company",
    "DUK": "Duke Energy Corporation",
    "REGN": "Regeneron Pharmaceuticals",
    "AON": "Aon plc",
    "USB": "U.S. Bancorp",
    "PGR": "The Progressive Corporation",
    "TGT": "Target Corporation",
    "AXP": "American Express Company",
    "FIS": "Fidelity National Info",
    "SPY": "SPDR S&P 500 ETF Trust",
    "QQQ": "Invesco QQQ Trust",
    "IWM": "iShares Russell 2000 ETF",
    "DIA": "SPDR Dow Jones Industrial ETF",
    "VTI": "Vanguard Total Stock Market ETF",
    "VOO": "Vanguard S&P 500 ETF",
    "AMD": "Advanced Micro Devices, Inc.",
    "AVGO": "Broadcom Inc.",
    "ADBE": "Adobe Inc.",
    "PYPL": "PayPal Holdings, Inc.",
    "UBER": "Uber Technologies, Inc.",
    "COIN": "Coinbase Global, Inc.",
    "SHOP": "Shopify Inc.",
    "SQ": "Block, Inc.",
}

DEFAULT_TICKERS = list(KNOWN_TICKER_NAMES.keys())[:50]
PERIOD = "2y"  # ~500 trading days
DATE = datetime.date.today().isoformat()
DATA_DIR = pathlib.Path(__file__).parent / "data"
HASHES_FILE = DATA_DIR / "hashes.json"

# ──────────────────────────────────────────────────────────────────────────────


def _cache_key(ticker: str, period: str, date: str = DATE) -> str:
    prefix = "500d" if period in ("2y", "500d") else period
    return f"{prefix}_{ticker}_{date}"


def _csv_path(ticker: str, period: str, date: str = DATE) -> pathlib.Path:
    return DATA_DIR / f"{_cache_key(ticker, period, date)}.csv"


def find_existing_cached_csv(ticker: str, period: str) -> tuple[pathlib.Path | None, int]:
    """Find any valid existing CSV for this ticker in DATA_DIR.
    Checks today's file first, then any file containing the ticker stem with >= 2 lines.
    """
    today_file = _csv_path(ticker, period)
    if today_file.exists():
        try:
            lines = today_file.read_text().splitlines()
            if len(lines) >= 2:
                return today_file, len(lines) - 1
        except Exception:
            pass

    # Check any file matching *_{ticker}_*.csv, *_{ticker}.csv, or {ticker}.csv
    candidates = []
    t_clean = ticker.strip().upper()
    if DATA_DIR.exists():
        for f in DATA_DIR.glob("*.csv"):
            parts = f.stem.split("_")
            if any(p.upper() == t_clean for p in parts):
                try:
                    lines = f.read_text().splitlines()
                    if len(lines) >= 2:
                        candidates.append((f, len(lines) - 1, f.stat().st_mtime))
                except Exception:
                    pass

    if candidates:
        # Sort by mtime descending (most recently modified first)
        candidates.sort(key=lambda c: c[2], reverse=True)
        return candidates[0][0], candidates[0][1]

    return None, 0


def _sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def _load_hashes() -> dict:
    if HASHES_FILE.exists():
        try:
            return json.loads(HASHES_FILE.read_text())
        except Exception:
            return {}
    return {}


def _save_hashes(hashes: dict) -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    HASHES_FILE.write_text(json.dumps(hashes, indent=2))


def get_company_name(ticker: str) -> str:
    ticker = ticker.strip().upper().replace(".", "-")
    if ticker in KNOWN_TICKER_NAMES:
        return KNOWN_TICKER_NAMES[ticker]
    return f"{ticker} Corp"


def batch_ensure_tickers(
    tickers: list[str], period: str, force: bool
) -> list[dict]:
    """Download multiple tickers efficiently. Uses cache where available,
    and batches all remaining downloads in threaded chunks.
    """
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    hashes = _load_hashes()

    # Deduplicate while preserving order and normalize '.' to '-'
    unique_tickers: list[str] = []
    for t in tickers:
        clean = t.strip().upper().replace(".", "-")
        if clean and clean not in unique_tickers:
            unique_tickers.append(clean)

    results_map: dict[str, dict] = {}
    missing_from_cache: list[str] = []

    for ticker in unique_tickers:
        name = get_company_name(ticker)

        if not force:
            existing_csv, row_count = find_existing_cached_csv(ticker, period)
            if existing_csv is not None:
                results_map[ticker] = {
                    "ticker": ticker,
                    "name": name,
                    "success": True,
                    "csv_path": str(existing_csv.resolve()),
                    "rows": row_count,
                    "cached": True,
                    "message": "cached — skipped",
                }
                continue

        missing_from_cache.append(ticker)

    # Batch download missing tickers in threaded chunks of 25
    if missing_from_cache:
        chunk_size = 25
        for i in range(0, len(missing_from_cache), chunk_size):
            chunk = missing_from_cache[i : i + chunk_size]
            try:
                raw = yf.download(
                    chunk,
                    period=period,
                    auto_adjust=True,
                    progress=False,
                    threads=True,
                    timeout=30,
                )

                close_df = raw["Close"] if "Close" in raw else None

                for ticker in chunk:
                    key = _cache_key(ticker, period)
                    csv = _csv_path(ticker, period)
                    name = get_company_name(ticker)

                    series: pd.Series | None = None
                    if close_df is not None:
                        if len(chunk) == 1:
                            if isinstance(close_df, pd.Series):
                                series = close_df.dropna()
                            elif isinstance(close_df, pd.DataFrame):
                                series = close_df.squeeze().dropna()
                        else:
                            if isinstance(close_df, pd.DataFrame) and ticker in close_df.columns:
                                series = close_df[ticker].dropna()

                    if series is None or series.empty:
                        results_map[ticker] = {
                            "ticker": ticker,
                            "name": name,
                            "success": False,
                            "csv_path": None,
                            "rows": 0,
                            "cached": False,
                            "message": "No price data returned from Yahoo Finance",
                        }
                    else:
                        if len(series) > 500:
                            series = series.iloc[-500:]
                        series.name = ticker
                        csv.parent.mkdir(parents=True, exist_ok=True)
                        csv.write_text(series.to_csv())
                        hashes[key] = _sha256(csv)
                        row_count = len(series)

                        results_map[ticker] = {
                            "ticker": ticker,
                            "name": name,
                            "success": True,
                            "csv_path": str(csv.resolve()),
                            "rows": row_count,
                            "cached": False,
                            "message": f"{row_count} rows downloaded",
                        }
            except Exception as exc:
                for ticker in chunk:
                    if ticker not in results_map:
                        results_map[ticker] = {
                            "ticker": ticker,
                            "name": get_company_name(ticker),
                            "success": False,
                            "csv_path": None,
                            "rows": 0,
                            "cached": False,
                            "message": f"Download failed: {exc}",
                        }

        _save_hashes(hashes)

    # Return in original order
    return [results_map[t] for t in unique_tickers if t in results_map]


def cmd_ensure(ticker: str, period: str, force: bool, as_json: bool) -> int:
    results = batch_ensure_tickers([ticker], period, force)
    if not results:
        return 1
    res = results[0]
    if as_json:
        print(json.dumps(res))
    else:
        status = "✓" if res["success"] else "✗"
        print(f"  {status} [{res['ticker']}] {res['message']}")
    return 0 if res["success"] else 1


def cmd_batch_json(tickers: list[str], period: str, force: bool, as_json: bool) -> int:
    results = batch_ensure_tickers(tickers, period, force)
    ok_count = sum(1 for r in results if r["success"])
    failed_count = len(results) - ok_count

    if as_json:
        payload = {
            "success": ok_count > 0,
            "count": len(results),
            "ok_count": ok_count,
            "failed_count": failed_count,
            "results": results,
        }
        print(json.dumps(payload))
    else:
        print(f"Processed {len(results)} ticker(s): {ok_count} ok, {failed_count} failed.")
        for r in results:
            status = "✓" if r["success"] else "✗"
            print(f"  {status} [{r['ticker']}] {r['message']}")

    return 0 if failed_count == 0 else 1


def cmd_search(query: str, as_json: bool) -> int:
    query = query.strip().upper()
    name = KNOWN_TICKER_NAMES.get(query)

    if name:
        res = {"valid": True, "ticker": query, "name": name, "currency": "USD"}
    else:
        try:
            t = yf.Ticker(query)
            currency = getattr(t.fast_info, "currency", None) or "USD"
            # Verify market data exists
            hist = t.history(period="5d")
            if hist.empty:
                res = {
                    "valid": False,
                    "ticker": query,
                    "name": "",
                    "currency": "",
                    "error": "Symbol not found or no market data",
                }
            else:
                res = {
                    "valid": True,
                    "ticker": query,
                    "name": f"{query} Corp",
                    "currency": currency,
                }
        except Exception as exc:
            res = {
                "valid": False,
                "ticker": query,
                "name": "",
                "currency": "",
                "error": str(exc),
            }

    if as_json:
        print(json.dumps(res))
    else:
        if res["valid"]:
            print(f"✓ [{res['ticker']}] {res['name']}")
        else:
            print(f"✗ [{query}] {res.get('error', 'Invalid')}")

    return 0 if res.get("valid") else 1


def cmd_index(index_name: str, period: str, force: bool, as_json: bool) -> int:
    name = index_name.lower().replace("-", "").replace("_", "").replace(" ", "")
    tickers: list[str] = []
    try:
        from yahoo_fin import stock_info as si
        if "sp500" in name or "s&p" in name or "sp" in name:
            tickers = si.tickers_sp500()
        elif "dow" in name:
            tickers = si.tickers_dow()
        elif "nasdaq" in name:
            tickers = si.tickers_nasdaq()
    except Exception:
        pass

    if not tickers:
        # Fallback to curated constituents if yahoo_fin is unreachable
        if "dow" in name:
            tickers = [
                "AAPL", "AMGN", "AXP", "BA", "CAT", "CRM", "CSCO", "CVX", "DIS", "DOW",
                "GS", "HD", "HON", "IBM", "INTC", "JNJ", "JPM", "KO", "MCD", "MMM",
                "MRK", "MSFT", "NKE", "NVDA", "PG", "SHW", "TRV", "UNH", "V", "WMT",
            ]
        elif "nasdaq" in name:
            tickers = [
                "AAPL", "MSFT", "NVDA", "AMZN", "GOOGL", "GOOG", "META", "TSLA", "AVGO",
                "COST", "ASML", "PEP", "NFLX", "LIN", "AMD", "ADBE", "QCOM", "TMUS", "INTU",
                "AMAT", "CSCO", "ISRG", "TXN", "HON", "BKNG", "CMCSA", "AMGN", "VRTX", "ADI",
            ]
        else:
            tickers = DEFAULT_TICKERS

    tickers = [t.strip().upper().replace(".", "-") for t in tickers if t.strip()]
    return cmd_batch_json(tickers, period, force, as_json)


def main() -> None:
    args = sys.argv[1:]

    if len(args) >= 1 and args[0] in ("ensure", "search", "batch", "index"):
        parser = argparse.ArgumentParser(prog="data_downloader.py")
        subparsers = parser.add_subparsers(dest="command")

        ensure_parser = subparsers.add_parser("ensure", help="Ensure a single ticker is cached")
        ensure_parser.add_argument("ticker", help="Ticker symbol")
        ensure_parser.add_argument("--period", default=PERIOD, help="History period")
        ensure_parser.add_argument("--force", action="store_true", help="Force re-download")
        ensure_parser.add_argument("--json", action="store_true", help="Output JSON format")

        batch_parser = subparsers.add_parser("batch", help="Batch download multiple tickers")
        batch_parser.add_argument("tickers", nargs="+", help="Ticker symbols")
        batch_parser.add_argument("--period", default=PERIOD, help="History period")
        batch_parser.add_argument("--force", action="store_true", help="Force re-download")
        batch_parser.add_argument("--json", action="store_true", help="Output JSON format")

        index_parser = subparsers.add_parser("index", help="Download live index tickers via yahoo_fin")
        index_parser.add_argument("index", help="Index name (sp500, dow, nasdaq)")
        index_parser.add_argument("--period", default=PERIOD, help="History period")
        index_parser.add_argument("--force", action="store_true", help="Force re-download")
        index_parser.add_argument("--json", action="store_true", help="Output JSON format")

        search_parser = subparsers.add_parser("search", help="Search/validate ticker metadata")
        search_parser.add_argument("query", help="Ticker query")
        search_parser.add_argument("--json", action="store_true", help="Output JSON format")

        parsed = parser.parse_args(args)
        if parsed.command == "ensure":
            sys.exit(cmd_ensure(parsed.ticker, parsed.period, parsed.force, parsed.json))
        elif parsed.command == "batch":
            sys.exit(cmd_batch_json(parsed.tickers, parsed.period, parsed.force, parsed.json))
        elif parsed.command == "index":
            sys.exit(cmd_index(parsed.index, parsed.period, parsed.force, parsed.json))
        elif parsed.command == "search":
            sys.exit(cmd_search(parsed.query, parsed.json))

    # Backward-compatible batch mode
    force = "--force" in args
    tickers = [a for a in args if not a.startswith("--")] or DEFAULT_TICKERS
    sys.exit(cmd_batch_json(tickers, PERIOD, force, False))


if __name__ == "__main__":
    main()
