require("dotenv").config();

const path = require("path");
const express = require("express");
const cookieParser = require("cookie-parser");
const jwt = require("jsonwebtoken");
const {
  ensureSchema,
  listTags,
  listPages,
  getPage,
  createPage,
  updatePage,
  deletePage,
} = require("./db");

const app = express();
const port = process.env.PORT || 3000;
const password = process.env.APP_PASSWORD || "";
const sessionSecret = process.env.SESSION_SECRET || "";

if (!password || !sessionSecret) {
  console.warn("APP_PASSWORD and SESSION_SECRET must be set in .env");
}

app.use(express.json({ limit: "5mb" }));
app.use(cookieParser());
app.use(express.static(path.join(__dirname, "..", "public")));

function issueToken() {
  return jwt.sign({ sub: "jourloc" }, sessionSecret, { expiresIn: "7d" });
}

function authMiddleware(req, res, next) {
  const token = req.cookies.jourloc_session;
  if (!token) {
    return res.status(401).json({ error: "Not authenticated" });
  }

  try {
    jwt.verify(token, sessionSecret);
    return next();
  } catch (error) {
    return res.status(401).json({ error: "Invalid session" });
  }
}

app.get("/api/health", (req, res) => {
  res.json({ status: "ok" });
});

app.get("/api/me", (req, res) => {
  const token = req.cookies.jourloc_session;
  if (!token) {
    return res.json({ authenticated: false });
  }

  try {
    jwt.verify(token, sessionSecret);
    return res.json({ authenticated: true });
  } catch (error) {
    return res.json({ authenticated: false });
  }
});

app.post("/api/login", (req, res) => {
  const { password: provided } = req.body || {};
  if (!provided || provided !== password) {
    return res.status(401).json({ error: "Invalid password" });
  }

  const token = issueToken();
  res.cookie("jourloc_session", token, {
    httpOnly: true,
    sameSite: "lax",
    maxAge: 7 * 24 * 60 * 60 * 1000,
  });
  return res.json({ authenticated: true });
});

app.post("/api/logout", (req, res) => {
  res.clearCookie("jourloc_session");
  res.json({ ok: true });
});

app.get("/api/pages", authMiddleware, async (req, res) => {
  try {
    const search = (req.query.search || "").trim();
    const tag = (req.query.tag || "").trim();
    const pages = await listPages({
      search: search || null,
      tag: tag || null,
    });
    res.json({ pages });
  } catch (error) {
    res.status(500).json({ error: "Failed to list pages" });
  }
});

app.get("/api/pages/:id", authMiddleware, async (req, res) => {
  try {
    const page = await getPage(Number(req.params.id));
    if (!page) {
      return res.status(404).json({ error: "Page not found" });
    }
    res.json({ page });
  } catch (error) {
    res.status(500).json({ error: "Failed to fetch page" });
  }
});

app.post("/api/pages", authMiddleware, async (req, res) => {
  try {
    const { title, content, tags } = req.body || {};
    if (!title || !title.trim()) {
      return res.status(400).json({ error: "Title is required" });
    }
    const page = await createPage({
      title: title.trim(),
      content: content || "",
      tags: tags || [],
    });
    res.json({ page });
  } catch (error) {
    res.status(500).json({ error: "Failed to create page" });
  }
});

app.put("/api/pages/:id", authMiddleware, async (req, res) => {
  try {
    const { title, content, tags } = req.body || {};
    if (!title || !title.trim()) {
      return res.status(400).json({ error: "Title is required" });
    }
    const page = await updatePage(Number(req.params.id), {
      title: title.trim(),
      content: content || "",
      tags: tags || [],
    });
    if (!page) {
      return res.status(404).json({ error: "Page not found" });
    }
    res.json({ page });
  } catch (error) {
    res.status(500).json({ error: "Failed to update page" });
  }
});

app.delete("/api/pages/:id", authMiddleware, async (req, res) => {
  try {
    const ok = await deletePage(Number(req.params.id));
    if (!ok) {
      return res.status(404).json({ error: "Page not found" });
    }
    res.json({ ok: true });
  } catch (error) {
    res.status(500).json({ error: "Failed to delete page" });
  }
});

app.get("/api/tags", authMiddleware, async (req, res) => {
  try {
    const tags = await listTags();
    res.json({ tags });
  } catch (error) {
    res.status(500).json({ error: "Failed to list tags" });
  }
});

app.get("*", (req, res) => {
  res.sendFile(path.join(__dirname, "..", "public", "index.html"));
});

ensureSchema()
  .then(() => {
    app.listen(port, "0.0.0.0", () => {
      console.log(`JourLoc listening on port ${port}`);
    });
  })
  .catch((error) => {
    console.error("Failed to initialize database", error);
    process.exit(1);
  });
