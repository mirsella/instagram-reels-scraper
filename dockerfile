FROM rust:1.74.1-alpine

WORKDIR /app

COPY . .

RUN apk add --no-cache chromium ca-certificates libc-dev
RUN cargo build --release

ENV RUSTLOG=debug

CMD ["./target/release/instagram-reels-scraper"]
