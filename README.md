# instagram-reels-scraper

Rust scraper that collects reels from configured Instagram accounts, ranks them, writes an `.ods` spreadsheet, and can send the result to Slack or Telegram.

## How It Works

The app logs into Instagram, spawns several scraping workers, and opens each account's reels page. Instead of scraping only visible HTML, it watches Instagram network responses, parses reel metadata from API payloads, and builds a normalized list of reels.

The final report is ranked by follower-normalized performance, then written to an OpenDocument spreadsheet with separate tabs for `today`, `yesterday`, and `last month`.

## Setup

```bash
cp .env.example .env
```

Fill `.env` with your Instagram credentials, account list, and optional Slack or Telegram settings.

## Run

```bash
cargo run
```

For a local-only test that keeps the generated file in the current directory:

```bash
cargo run --features dryrun
```

## Build

```bash
cargo build --release
```

## Notes

- `dryrun` keeps the generated file locally as `./reels.ods` instead of uploading it.
- Slack and Telegram are optional, but the environment file supports both.
- The default scraping window is recent content unless you build/run with extra features.
