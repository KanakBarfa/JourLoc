# JourLoc — Secure Markdown Journal

JourLoc is a self-hosted, lightweight, and secure markdown journaling web application for keeping personal logs, notes, and equations. It is designed to be self-hosted on a local network (LAN) and uses a single secure shared password, allowing any device in your home/office to unlock and sync with the same journal.

<img src="./resources/image.png" alt="JourLoc" style="max-width:800px; width:100%; height:auto; border-radius: 12px; box-shadow: 0 10px 30px rgba(0,0,0,0.3);" />

---

## 🌟 Key Features

* **Rich Obsidian-First Theme**: A gorgeous glassmorphic dark user interface built with Outfit, Inter, and Lora typography.
* **Dual-Pane Markdown Workspace**: Interactive live preview pane alongside a distraction-free markdown text editor.
* **MathJax Support**: Render complex mathematical equations locally using standard `$$ ... $$` block or `$ ... $` inline markup.
* **Flexible Page Resizers**: Tactile drag-and-resize panels for both the sidebar and the editor workspace with hover indicators.
* **Tagging & Filtering**: Quick tag management (adding, filtering) and title search functionality.
* **Safe Session Management**: Session validation using JWT and standard secure cookies, with an interactive **Log out** portal that clears browser states.

---

## 🏗️ Architecture & How It Works

* **Backend (Rust)**: Built on top of **Axum 0.7** and **SQLx 0.7**. Restructured into clean, isolated modules:
  * [src/main.rs](file:///home/kanak/JourLoc/src/main.rs): Setup & router registration.
  * [src/config.rs](file:///home/kanak/JourLoc/src/config.rs): Decoupled environment loader.
  * [src/error.rs](file:///home/kanak/JourLoc/src/error.rs): Custom `AppError` implementing Axum `IntoResponse` for centralized error responses.
  * [src/auth.rs](file:///home/kanak/JourLoc/src/auth.rs): Type-safe `Claims` request extractor using Axum extractor guards.
  * [src/db.rs](file:///home/kanak/JourLoc/src/db.rs): SQLx execution layer.
  * [src/handlers.rs](file:///home/kanak/JourLoc/src/handlers.rs): Modular controller functions.
* **Database (PostgreSQL)**: Handles persistent storage of journal pages, tags, and mapping associations. Auto-cleans orphaned tags using SQL triggers on schema deletion.
* **Frontend**: HTML5, Vanilla JavaScript, and modern CSS variables utilizing backdrop filters and transition states. Markdown parsing and HTML sanitization are handled client-side using `marked.js` and `DOMPurify` respectively.

---

## ⚙️ Environment Variables

Configure these settings inside a `.env` file at the project root:

| Variable | Description | Example |
| :--- | :--- | :--- |
| `APP_PASSWORD` | Shared login password to decrypt/unlock journal entries. | `secret123` |
| `SESSION_SECRET` | Secret key used to sign and validate session JWT cookies. | `use-a-long-random-secret-string` |
| `DATABASE_URL` | PostgreSQL connection connection URI. | `postgres://user:pass@host:5432/db` |
| `PORT` | Local network port the server listens on (defaults to 3000). | `3000` |

---

## 🐳 Quick Start with Docker

1. Create your `.env` file from the variables above. Set `DATABASE_URL` to point to the docker container service name:
   ```env
   DATABASE_URL=postgres://jourloc:jourloc@db:5432/jourloc
   ```
2. Build and start the services:
   ```bash
   docker compose up --build
   ```
3. Open `http://localhost:3000` in your web browser.

---

## 💻 Local Development Setup

To run JourLoc directly on your workstation:

1. **Install Frontend Dependencies**:
   Copy third-party vendor assets into the public directories:
   ```bash
   npm install --omit=dev
   mkdir -p public/vendor/mathjax/es5
   cp node_modules/marked/marked.min.js public/vendor/
   cp node_modules/dompurify/dist/purify.min.js public/vendor/
   cp -R node_modules/mathjax/es5/. public/vendor/mathjax/es5/
   ```

2. **Database Setup**: Ensure PostgreSQL is running locally and run a query to create the database:
   ```sql
   CREATE DATABASE jourloc;
   ```

3. **Configure Local Environment**: Set `DATABASE_URL` in `.env` to point to localhost:
   ```env
   DATABASE_URL=postgres://username:password@localhost:5432/jourloc
   ```

4. **Launch the backend server**:
   ```bash
   cargo run
   ```
   Open `http://localhost:3000` in your web browser.
