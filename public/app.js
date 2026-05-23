const state = {
  pages: [],
  tags: [],
  selectedPage: null,
  search: "",
  tagFilter: "",
};

const elements = {
  pageList: document.getElementById("pageList"),
  tagList: document.getElementById("tagList"),
  titleInput: document.getElementById("titleInput"),
  tagsInput: document.getElementById("tagsInput"),
  contentInput: document.getElementById("contentInput"),
  preview: document.getElementById("preview"),
  status: document.getElementById("status"),
  searchInput: document.getElementById("searchInput"),
  newPage: document.getElementById("newPage"),
  savePage: document.getElementById("savePage"),
  deletePage: document.getElementById("deletePage"),
  loginOverlay: document.getElementById("loginOverlay"),
  loginButton: document.getElementById("loginButton"),
  passwordInput: document.getElementById("passwordInput"),
  loginError: document.getElementById("loginError"),
  sidebarToggle: document.getElementById("sidebarToggle"),
  sidebar: document.getElementById("sidebar"),
};

async function apiFetch(path, options = {}) {
  const response = await fetch(path, {
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include",
    ...options,
  });

  if (response.status === 401) {
    showLogin();
    return null;
  }

  return response.json();
}

function showLogin() {
  elements.loginOverlay.classList.remove("hidden");
}

function hideLogin() {
  elements.loginOverlay.classList.add("hidden");
  elements.loginError.textContent = "";
  elements.passwordInput.value = "";
}

function setStatus(message) {
  elements.status.textContent = message || "";
}

function renderPages() {
  elements.pageList.innerHTML = "";
  state.pages.forEach((page) => {
    const item = document.createElement("div");
    item.className = "page-item" + (state.selectedPage?.id === page.id ? " active" : "");
    item.textContent = page.title;
    item.addEventListener("click", () => selectPage(page.id));
    elements.pageList.appendChild(item);
  });
}

function renderTags() {
  elements.tagList.innerHTML = "";
  state.tags.forEach((tag) => {
    const item = document.createElement("div");
    item.className = "tag" + (state.tagFilter === tag ? " active" : "");
    item.textContent = tag;
    item.addEventListener("click", () => {
      state.tagFilter = state.tagFilter === tag ? "" : tag;
      loadPages();
      renderTags();
    });
    elements.tagList.appendChild(item);
  });
}

function clearEditor() {
  state.selectedPage = null;
  elements.titleInput.value = "";
  elements.tagsInput.value = "";
  elements.contentInput.value = "";
  renderPreview();
  setStatus("New page");
}

async function loadTags() {
  const data = await apiFetch("/api/tags");
  if (!data) return;
  state.tags = data.tags;
  renderTags();
}

async function loadPages() {
  const params = new URLSearchParams();
  if (state.search) params.set("search", state.search);
  if (state.tagFilter) params.set("tag", state.tagFilter);

  const data = await apiFetch(`/api/pages?${params.toString()}`);
  if (!data) return;
  state.pages = data.pages;
  renderPages();
  if (state.selectedPage) {
    const stillThere = state.pages.find((p) => p.id === state.selectedPage.id);
    if (!stillThere) {
      clearEditor();
    }
  }
}

async function selectPage(id) {
  const data = await apiFetch(`/api/pages/${id}`);
  if (!data) return;
  state.selectedPage = data.page;
  elements.titleInput.value = data.page.title;
  elements.tagsInput.value = (data.page.tags || []).join(", ");
  elements.contentInput.value = data.page.content || "";
  renderPreview();
  setStatus(`Editing page #${data.page.id}`);
  renderPages();
}

async function saveCurrentPage() {
  const payload = {
    title: elements.titleInput.value.trim(),
    content: elements.contentInput.value,
    tags: elements.tagsInput.value
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0),
  };

  if (!payload.title) {
    setStatus("Title is required");
    return;
  }

  let data = null;
  if (state.selectedPage?.id) {
    data = await apiFetch(`/api/pages/${state.selectedPage.id}`, {
      method: "PUT",
      body: JSON.stringify(payload),
    });
  } else {
    data = await apiFetch("/api/pages", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  if (!data) return;
  state.selectedPage = data.page;
  await loadPages();
  await loadTags();
  setStatus("Saved");
}

async function deleteCurrentPage() {
  if (!state.selectedPage?.id) {
    setStatus("Select a page first");
    return;
  }

  const ok = window.confirm("Delete this page? This cannot be undone.");
  if (!ok) return;

  const data = await apiFetch(`/api/pages/${state.selectedPage.id}`, {
    method: "DELETE",
  });
  if (!data) return;
  clearEditor();
  await loadPages();
  await loadTags();
  setStatus("Deleted");
}

async function login() {
  const password = elements.passwordInput.value;
  if (!password) {
    elements.loginError.textContent = "Password is required";
    return;
  }

  const response = await fetch("/api/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ password }),
  });

  if (!response.ok) {
    elements.loginError.textContent = "Wrong password";
    return;
  }

  hideLogin();
  await loadPages();
  await loadTags();
}

function renderPreview() {
  if (!elements.preview) return;
  if (!window.marked || !window.DOMPurify) {
    elements.preview.textContent = elements.contentInput.value;
    return;
  }
  const raw = window.marked.parse(elements.contentInput.value || "");
  elements.preview.innerHTML = window.DOMPurify.sanitize(raw);
}

async function init() {
  const me = await apiFetch("/api/me");
  if (!me || !me.authenticated) {
    showLogin();
    return;
  }

  hideLogin();
  await loadPages();
  await loadTags();
}

function bindEvents() {
  elements.newPage.addEventListener("click", clearEditor);
  elements.savePage.addEventListener("click", saveCurrentPage);
  elements.deletePage.addEventListener("click", deleteCurrentPage);
  elements.contentInput.addEventListener("input", renderPreview);
  elements.searchInput.addEventListener("input", (event) => {
    state.search = event.target.value.trim();
    loadPages();
  });
  elements.loginButton.addEventListener("click", login);
  elements.sidebarToggle.addEventListener("click", () => {
    elements.sidebar.classList.toggle("sidebar--collapsed");
  });
}

bindEvents();
init();
