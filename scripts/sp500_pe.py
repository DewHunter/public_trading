# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "pandas",
#     "yfinance",
# ]
# ///

"""
Fetch trailing P/E + recent price for every S&P 500 stock and sort by P/E.

Data source: Yahoo Finance via the `yfinance` library (free, no API key required).
Tickers come from a static list extracted from Wikipedia's S&P 500 page.

Notes on trailing vs. forward P/E:
  - Trailing P/E = price / (last 12 months of actual reported EPS).
    Backward-looking but factual.
  - Forward P/E = price / (analyst-estimated next-12-month EPS).
    Forward-looking but a guess.
  When people say "P/E ratio" they usually mean trailing, so I use Yahoo's
  `trailingPE` field. Swap to `forwardPE` if you'd rather.

Negative trailing P/E means the company lost money in the last 12 months.
Sorting those alongside positives is misleading (a stock with EPS of -$0.01
would show up as "cheapest" with a huge negative ratio), so they're separated.
"""

import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

import pandas as pd
import yfinance as yf
from tickers_list import TICKERS


def fetch_one(ticker: str) -> dict | None:
    try:
        info = yf.Ticker(ticker).info
        price = info.get("currentPrice") or info.get("regularMarketPrice")
        pe = info.get("trailingPE")
        if price is None:
            return None
        return {
            "ticker": ticker,
            "name": info.get("shortName") or info.get("longName") or "",
            "sector": info.get("sector", ""),
            "price": price,
            "trailing_pe": pe,
            "mcap_b": (info.get("marketCap") or 0) / 1e9,
        }
    except Exception:
        return None


def main():
    print(f"Fetching {len(TICKERS)} tickers from Yahoo Finance...", file=sys.stderr)
    t0 = time.time()
    rows, failed = [], []

    with ThreadPoolExecutor(max_workers=20) as ex:
        futures = {ex.submit(fetch_one, t): t for t in TICKERS}
        for i, fut in enumerate(as_completed(futures), 1):
            t = futures[fut]
            r = fut.result()
            if r is None:
                failed.append(t)
            else:
                rows.append(r)
            if i % 50 == 0:
                print(
                    f"  ...{i}/{len(TICKERS)} ({time.time() - t0:.0f}s)",
                    file=sys.stderr,
                )

    print(
        f"\nDone in {time.time() - t0:.0f}s. {len(rows)} ok, {len(failed)} failed.",
        file=sys.stderr,
    )
    if failed:
        print(
            f"Failed: {failed[:20]}{'...' if len(failed) > 20 else ''}", file=sys.stderr
        )

    df = pd.DataFrame(rows)
    positive = df[df["trailing_pe"].notna() & (df["trailing_pe"] > 0)].copy()
    positive = positive.sort_values("trailing_pe").reset_index(drop=True)
    positive.insert(0, "rank", range(1, len(positive) + 1))
    no_pe = df[df["trailing_pe"].isna()].copy()
    negative_pe = df[df["trailing_pe"].notna() & (df["trailing_pe"] <= 0)].copy()

    positive.to_csv("sp500_by_pe.csv", index=False)

    pd.set_option("display.max_rows", None)
    pd.set_option("display.width", 160)
    pd.set_option("display.max_colwidth", 35)
    pd.set_option("display.float_format", lambda x: f"{x:,.2f}")

    print("\n" + "=" * 90)
    print(
        f"S&P 500 sorted by trailing P/E (ascending) — {len(positive)} stocks with positive P/E"
    )
    print("=" * 90)
    print(positive.to_string(index=False))

    if len(negative_pe):
        print("\n" + "=" * 90)
        print(f"Unprofitable (negative trailing P/E): {len(negative_pe)} stocks")
        print("=" * 90)
        print(
            negative_pe[["ticker", "name", "sector", "price", "trailing_pe"]].to_string(
                index=False
            )
        )

    if len(no_pe):
        print("\n" + "=" * 90)
        print(f"No P/E reported: {len(no_pe)} stocks")
        print("=" * 90)
        print(no_pe[["ticker", "name", "sector", "price"]].to_string(index=False))


if __name__ == "__main__":
    main()
