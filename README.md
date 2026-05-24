# JourLoc

JourLoc is a vibe coded journaling web app for keeping my personal notes on a local network. It is designed for one shared password, not multiple user accounts, so you can open it from any device on your LAN and keep your journal data in one place.

## What it does
- Create, edit, and delete journal pages.
- Write page content in Markdown.
- Add tags to pages and filter by tag.
- Search pages by title.
- Preview Markdown and math equations in the browser.
- Use a collapsible sidebar to navigate pages quickly.

## How it works
- The app runs as a single Node.js service with an Express API and frontend.
- Data is stored in Postgres.
- The app is intended to be run in Docker and configured with environment variables.
- The client is kept lightweight and the app works offline after the image is built.

## Environment variables
Create a `.env` file from `.env.example` and set:
- `APP_PASSWORD`: the shared login password
- `SESSION_SECRET`: secret used to sign the login session cookie
- `DATABASE_URL`: Postgres connection string
- `PORT`: port the app listens on

## Quick start with Docker
1. Copy `.env.example` to `.env` and update the values.
2. Start the stack:

```bash
docker compose up --build
```

3. Open the app at `http://localhost:3000`.

## Development
1. Make sure Postgres is running and `DATABASE_URL` points to it.
2. Install dependencies:

```bash
npm install
```

3. Start the dev server:

```bash
npm run dev
```

## Notes
- The database is meant to be persisted with a Docker volume so data survives image rebuilds and redeploys.
- The app currently uses a single shared password instead of individual user accounts.
- Markdown and math rendering are handled locally, so the app does not need external APIs after the image is built.
