FROM node:20-alpine AS browser-assets
WORKDIR /assets

COPY package.json ./
RUN npm install --omit=dev

RUN mkdir -p /out/vendor/mathjax/es5 \
  && cp node_modules/marked/marked.min.js /out/vendor/ \
  && cp node_modules/dompurify/dist/purify.min.js /out/vendor/ \
  && cp -R node_modules/mathjax/es5/. /out/vendor/mathjax/es5/

FROM rust:1.88-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY migrations ./migrations
COPY src ./src
COPY public ./public
COPY --from=browser-assets /out/vendor ./public/vendor

RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN useradd --create-home --shell /usr/sbin/nologin jourloc

COPY --from=builder /app/target/release/jourloc /usr/local/bin/jourloc
COPY --from=builder /app/public ./public

ENV PORT=3000
EXPOSE 3000

USER jourloc
CMD ["jourloc"]
