FROM rust:1.78.0-alpine

WORKDIR /app

COPY . .

RUN apk add --no-cache chromium ca-certificates libc-dev tzdata
RUN cargo build --release

ENV RUSTLOG=debug
ENV TZ=Europe/Paris

CMD ["./target/release/instagram-reels-scraper"]
