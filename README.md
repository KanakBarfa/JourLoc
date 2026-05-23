# JourLoc

Local journaling web app with markdown pages, tags, and a single shared password.

## Features
- Flat list of pages with title, tags, and markdown content
- Search by title and filter by tag
- Single-password login for local network usage
- Dockerized deployment with Postgres persistence

## Quick start (Docker)
1. Copy `.env.example` to `.env` and set `APP_PASSWORD` and `SESSION_SECRET`.
2. Run: `docker compose up --build`
3. Open `http://localhost:3000`

## Dev (local)
1. Start Postgres and set `DATABASE_URL`
2. `npm install`
3. `npm run dev`
