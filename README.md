# instagram-reels-scraper

Rust scraper that collects reels from configured Instagram accounts, ranks them, writes an `.ods` spreadsheet, and can send the result to Slack or Telegram.

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
