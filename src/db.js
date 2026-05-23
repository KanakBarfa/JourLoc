const fs = require("fs");
const path = require("path");
const { Pool } = require("pg");

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

async function ensureSchema() {
  const schemaPath = path.join(__dirname, "schema.sql");
  const sql = fs.readFileSync(schemaPath, "utf8");
  await pool.query(sql);
}

function normalizeTags(tags) {
  if (!Array.isArray(tags)) return [];
  const normalized = tags
    .map((t) => (t || "").trim())
    .filter((t) => t.length > 0);
  return Array.from(new Set(normalized));
}

async function listTags() {
  const result = await pool.query("SELECT name FROM tags ORDER BY name ASC");
  return result.rows.map((row) => row.name);
}

async function listPages({ search, tag }) {
  const params = [];
  let whereClauses = [];

  if (search) {
    params.push(`%${search}%`);
    whereClauses.push(`p.title ILIKE $${params.length}`);
  }

  if (tag) {
    params.push(tag);
    whereClauses.push(
      `EXISTS (
        SELECT 1 FROM page_tags pt2
        JOIN tags t2 ON t2.id = pt2.tag_id
        WHERE pt2.page_id = p.id AND t2.name = $${params.length}
      )`
    );
  }

  const whereSql = whereClauses.length ? `WHERE ${whereClauses.join(" AND ")}` : "";

  const query = `
    SELECT p.id, p.title, p.updated_at, p.created_at,
           COALESCE(t.tags, '[]'::json) AS tags
    FROM pages p
    LEFT JOIN (
      SELECT pt.page_id, JSON_AGG(t.name ORDER BY t.name) AS tags
      FROM page_tags pt
      JOIN tags t ON t.id = pt.tag_id
      GROUP BY pt.page_id
    ) t ON t.page_id = p.id
    ${whereSql}
    ORDER BY p.updated_at DESC
  `;

  const result = await pool.query(query, params);
  return result.rows;
}

async function getPage(id) {
  const query = `
    SELECT p.id, p.title, p.content, p.updated_at, p.created_at,
           COALESCE(t.tags, '[]'::json) AS tags
    FROM pages p
    LEFT JOIN (
      SELECT pt.page_id, JSON_AGG(t.name ORDER BY t.name) AS tags
      FROM page_tags pt
      JOIN tags t ON t.id = pt.tag_id
      GROUP BY pt.page_id
    ) t ON t.page_id = p.id
    WHERE p.id = $1
  `;

  const result = await pool.query(query, [id]);
  return result.rows[0] || null;
}

async function createPage({ title, content, tags }) {
  const client = await pool.connect();
  const normalizedTags = normalizeTags(tags);

  try {
    await client.query("BEGIN");
    const insert = await client.query(
      "INSERT INTO pages (title, content) VALUES ($1, $2) RETURNING id",
      [title, content]
    );
    const pageId = insert.rows[0].id;
    await replaceTags(client, pageId, normalizedTags);
    await client.query("COMMIT");
    return await fetchPageById(client, pageId);
  } catch (error) {
    await client.query("ROLLBACK");
    throw error;
  } finally {
    client.release();
  }
}

async function updatePage(id, { title, content, tags }) {
  const client = await pool.connect();
  const normalizedTags = normalizeTags(tags);

  try {
    await client.query("BEGIN");
    const update = await client.query(
      "UPDATE pages SET title = $1, content = $2, updated_at = NOW() WHERE id = $3",
      [title, content, id]
    );
    if (update.rowCount === 0) {
      await client.query("ROLLBACK");
      return null;
    }
    await replaceTags(client, id, normalizedTags);
    await client.query("COMMIT");
    return await fetchPageById(client, id);
  } catch (error) {
    await client.query("ROLLBACK");
    throw error;
  } finally {
    client.release();
  }
}

async function deletePage(id) {
  const result = await pool.query("DELETE FROM pages WHERE id = $1", [id]);
  return result.rowCount > 0;
}

async function replaceTags(client, pageId, tags) {
  await client.query("DELETE FROM page_tags WHERE page_id = $1", [pageId]);

  for (const tag of tags) {
    const insertTag = await client.query(
      "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
      [tag]
    );
    const tagId = insertTag.rows[0].id;
    await client.query(
      "INSERT INTO page_tags (page_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
      [pageId, tagId]
    );
  }
}

async function fetchPageById(client, id) {
  const query = `
    SELECT p.id, p.title, p.content, p.updated_at, p.created_at,
           COALESCE(t.tags, '[]'::json) AS tags
    FROM pages p
    LEFT JOIN (
      SELECT pt.page_id, JSON_AGG(t.name ORDER BY t.name) AS tags
      FROM page_tags pt
      JOIN tags t ON t.id = pt.tag_id
      GROUP BY pt.page_id
    ) t ON t.page_id = p.id
    WHERE p.id = $1
  `;

  const result = await client.query(query, [id]);
  return result.rows[0] || null;
}

module.exports = {
  pool,
  ensureSchema,
  listTags,
  listPages,
  getPage,
  createPage,
  updatePage,
  deletePage,
};
