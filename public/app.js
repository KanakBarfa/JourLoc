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
  sidebarResizer: document.getElementById("sidebarResizer"),
  editor: document.getElementById("editor"),
  editorResizer: document.getElementById("editorResizer"),
  editorLeft: document.getElementById("editorLeft"),
  editorRight: document.getElementById("editorRight"),
  toggleEditor: document.getElementById("toggleEditor"),
  app: document.getElementById("app"),
  logoutButton: document.getElementById("logoutButton"),
};

async function apiFetch(path, options = {}) {
  try {
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
  } catch (err) {
    console.error("API error:", err);
    setStatus("Network error. Check connection.");
    return null;
  }
}

function showLogin() {
  elements.loginOverlay.classList.remove("hidden");
  // Clean memory state for security
  state.pages = [];
  state.tags = [];
  state.selectedPage = null;
  state.search = "";
  state.tagFilter = "";
  clearEditor();
  renderPages();
  renderTags();
}

function hideLogin() {
  elements.loginOverlay.classList.add("hidden");
  elements.loginError.textContent = "";
  elements.passwordInput.value = "";
}

function setStatus(message) {
  elements.status.textContent = message || "";
  // Fade out status after some time if it's a notification
  if (message && message !== "New page" && !message.startsWith("Editing page")) {
    setTimeout(() => {
      if (elements.status.textContent === message) {
        if (state.selectedPage) {
          setStatus(`Editing page #${state.selectedPage.id}`);
        } else {
          setStatus("New page");
        }
      }
    }, 4000);
  }
}

function renderPages() {
  elements.pageList.innerHTML = "";
  if (state.pages.length === 0) {
    const placeholder = document.createElement("div");
    placeholder.className = "sidebar-placeholder";
    placeholder.textContent = "No pages found";
    elements.pageList.appendChild(placeholder);
    return;
  }

  state.pages.forEach((page) => {
    const item = document.createElement("div");
    item.className = "page-item" + (state.selectedPage?.id === page.id ? " active" : "");
    
    // Create modern item content with subtle icon & meta details
    item.innerHTML = `
      <div class="page-item__title">${escapeHtml(page.title)}</div>
      <div class="page-item__date">${formatDate(page.updated_at || page.created_at)}</div>
    `;
    item.addEventListener("click", () => selectPage(page.id));
    elements.pageList.appendChild(item);
  });
}

function renderTags() {
  elements.tagList.innerHTML = "";
  if (state.tags.length === 0) {
    const placeholder = document.createElement("div");
    placeholder.className = "sidebar-placeholder";
    placeholder.textContent = "No tags";
    elements.tagList.appendChild(placeholder);
    return;
  }

  state.tags.forEach((tag) => {
    const item = document.createElement("div");
    item.className = "tag" + (state.tagFilter === tag ? " active" : "");
    item.innerHTML = `
      <span class="tag-hash">#</span>
      <span class="tag-name">${escapeHtml(tag)}</span>
    `;
    item.addEventListener("click", () => {
      state.tagFilter = state.tagFilter === tag ? "" : tag;
      loadPages();
      renderTags();
    });
    elements.tagList.appendChild(item);
  });
}

function escapeHtml(text) {
  if (!text) return "";
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function formatDate(dateStr) {
  if (!dateStr) return "";
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch (e) {
    return dateStr;
  }
}

function clearEditor() {
  state.selectedPage = null;
  elements.titleInput.value = "";
  elements.tagsInput.value = "";
  elements.contentInput.value = "";
  renderPreview();
  setStatus("New page");
  renderPages();
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
  const title = elements.titleInput.value.trim();
  const payload = {
    title: title || "Untitled entry",
    content: elements.contentInput.value,
    tags: elements.tagsInput.value
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0),
  };

  if (!title) {
    setStatus("Title is required");
    elements.titleInput.focus();
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
  setStatus("Saved successfully");
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
  setStatus("Deleted page");
}

async function login() {
  const password = elements.passwordInput.value;
  if (!password) {
    elements.loginError.textContent = "Password is required";
    return;
  }

  elements.loginButton.textContent = "Unlocking...";
  elements.loginButton.disabled = true;

  try {
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
  } catch (err) {
    elements.loginError.textContent = "Failed to connect to server";
  } finally {
    elements.loginButton.textContent = "Unlock Journal";
    elements.loginButton.disabled = false;
  }
}

async function logout() {
  const response = await fetch("/api/logout", {
    method: "POST",
  });
  if (response.ok) {
    showLogin();
  }
}

function renderPreview() {
  if (!elements.preview) return;
  if (!window.marked || !window.DOMPurify) {
    elements.preview.textContent = elements.contentInput.value;
    return;
  }
  let md = elements.contentInput.value || "";

  // First, convert markdown to HTML
  const raw = window.marked.parse(md);

  // Sanitize HTML
  const clean = window.DOMPurify.sanitize(raw);

  // Insert into preview container
  elements.preview.innerHTML = clean;
  // Ask MathJax to typeset the preview area if available
  if (window.MathJax && window.MathJax.typesetPromise) {
    try {
      window.MathJax.typesetPromise([elements.preview]).catch(() => {});
    } catch (e) {
      // ignore
    }
  }
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
  
  if (elements.logoutButton) {
    elements.logoutButton.addEventListener("click", logout);
  }

  // Toggle editor visibility
  elements.toggleEditor.addEventListener("click", () => {
    const collapsed = elements.app.classList.toggle("editor-collapsed");
    elements.toggleEditor.title = collapsed ? "Show editor" : "Hide editor";
    const spanText = elements.toggleEditor.querySelector("span");
    const svgIcon = elements.toggleEditor.querySelector("svg");
    
    if (spanText) {
      spanText.textContent = collapsed ? "Show editor" : "Hide editor";
    }
    if (svgIcon) {
      svgIcon.style.transform = collapsed ? "rotate(180deg)" : "rotate(0deg)";
    }
    // re-run MathJax typeset after layout change
    setTimeout(() => renderPreview(), 50);
  });

  // Sidebar resizer drag
  if (elements.sidebarResizer && elements.sidebar) {
    let isDragging = false;
    let startX = 0;
    let startWidth = 0;
    
    elements.sidebarResizer.addEventListener("mousedown", (e) => {
      isDragging = true;
      startX = e.clientX;
      startWidth = elements.sidebar.offsetWidth;
      document.body.style.userSelect = "none";
      elements.sidebarResizer.classList.add("resizer--active");
    });
    
    document.addEventListener("mousemove", (e) => {
      if (!isDragging) return;
      const dx = e.clientX - startX;
      let newWidth = startWidth + dx;
      const min = 200;
      const max = Math.min(600, window.innerWidth - 300);
      newWidth = Math.max(min, Math.min(max, newWidth));
      elements.sidebar.style.width = `${newWidth}px`;
    });
    
    document.addEventListener("mouseup", () => {
      if (isDragging) {
        isDragging = false;
        document.body.style.userSelect = "";
        elements.sidebarResizer.classList.remove("resizer--active");
      }
    });
  }

  // Editor resizer drag
  if (elements.editorResizer && elements.editorLeft && elements.editorRight && elements.editor) {
    let dragging = false;
    let startX = 0;
    let leftStart = 0;
    let rightStart = 0;
    
    elements.editorResizer.addEventListener("mousedown", (e) => {
      dragging = true;
      startX = e.clientX;
      leftStart = elements.editorLeft.getBoundingClientRect().width;
      rightStart = elements.editorRight.getBoundingClientRect().width;
      document.body.style.userSelect = "none";
      elements.editorResizer.classList.add("resizer--active");
    });
    
    document.addEventListener("mousemove", (e) => {
      if (!dragging) return;
      const dx = e.clientX - startX;
      let newLeft = leftStart + dx;
      let newRight = Math.max(200, rightStart - dx);
      const minLeft = 200;
      const minRight = 200;
      if (newLeft < minLeft) newLeft = minLeft;
      if (newRight < minRight) newRight = minRight;
      elements.editor.style.gridTemplateColumns = `${newLeft}px 8px ${newRight}px`;
    });
    
    document.addEventListener("mouseup", () => {
      if (dragging) {
        dragging = false;
        document.body.style.userSelect = "";
        elements.editorResizer.classList.remove("resizer--active");
      }
    });
  }

  elements.searchInput.addEventListener("input", (event) => {
    state.search = event.target.value.trim();
    loadPages();
  });
  
  elements.passwordInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      login();
    }
  });

  elements.loginButton.addEventListener("click", login);
  
  elements.sidebarToggle.addEventListener("click", () => {
    elements.sidebar.classList.toggle("sidebar--collapsed");
  });
}

bindEvents();
init();
