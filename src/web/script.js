// -----------------------------------------------------------------------
// State
// -----------------------------------------------------------------------

const params = new URLSearchParams(location.search);
const token = params.get("t");
const kind = params.get("kind") === "resourcepack" ? "resourcepack" : "mod";
const selectedTags = new Set();
const installed = new Set(); // id проектов, установленных за эту сессию

const $ = (id) => document.getElementById(id);

// -----------------------------------------------------------------------
// Small DOM/API helpers
// -----------------------------------------------------------------------

function api(path, extra = {}, method = "GET") {
  const p = new URLSearchParams({ t: token, kind, ...extra });
  return fetch(`${path}?${p}`, { method }).then((r) => r.json());
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

// -----------------------------------------------------------------------
// Modal (shared by project details and install picker)
// -----------------------------------------------------------------------

function closeModal() {
  $("overlay").classList.add("hidden");
  $("modal").replaceChildren();
}

function openModal(content) {
  $("modal").replaceChildren(content);
  $("overlay").classList.remove("hidden");
}

$("overlay").onclick = (e) => {
  if (e.target.id === "overlay") closeModal();
};

async function openProject(id) {
  openModal(el("div", "", "loading..."));
  try {
    const p = await api("/api/project", { id });
    if (p.error) throw new Error(p.error);

    const box = el("div");
    const closeBtn = el("button", "close", "close");
    closeBtn.onclick = closeModal;
    box.append(closeBtn, el("h2", "", p.title));

    if (p.gallery.length) {
      const gallery = el("div", "gallery");
      for (const shot of p.gallery) {
        const img = el("img");
        img.src = shot.url;
        img.alt = shot.title || "";
        gallery.append(img);
      }
      box.append(gallery);
    }

    box.append(el("pre", "", p.body || "(no description)"));
    openModal(box);
  } catch (e) {
    openModal(el("div", "", `error: ${e.message}`));
  }
}

async function openInstallPicker(projectId) {
  openModal(el("div", "", "loading versions..."));
  try {
    const all = await api("/api/targets");
    // для модов оставляем только версии с загрузчиком (fabric), vanilla прячем
    const targets = kind === "mod" ? all.filter((t) => t.loader) : all;
    if (!targets.length) {
      throw new Error(
        kind === "mod"
          ? "no modded versions installed (install Fabric in the launcher first)"
          : "no installed versions found"
      );
    }

    const box = el("div");
    const closeBtn = el("button", "close", "close");
    closeBtn.onclick = closeModal;
    box.append(closeBtn, el("h2", "", "Install for which version?"));

    const list = el("div", "targets");
    for (const t of targets) {
      const btn = el("button", "", t.label);
      btn.onclick = () => installTo(projectId, t, btn);
      list.append(btn);
    }
    box.append(list);
    openModal(box);
  } catch (e) {
    openModal(el("div", "", `error: ${e.message}`));
  }
}

async function installTo(projectId, target, btn) {
  btn.disabled = true;
  btn.textContent = "installing...";
  try {
    const r = await api("/api/install", { id: projectId, mc: target.mc, loader: target.loader || "" }, "POST");
    btn.textContent = r.ok ? "installed" : "failed";
    $("msg").textContent = r.ok ? `installed: ${r.files.join(", ")}` : `error: ${r.error}`;
    if (r.ok) {
      markInstalled(projectId);
      setTimeout(closeModal, 600);
    }
  } catch {
    btn.textContent = "failed";
    $("msg").textContent = "error: launcher not reachable";
  }
}

// -----------------------------------------------------------------------
// Result list
// -----------------------------------------------------------------------

function renderItem(hit) {
  const item = el("div", "item");

  const img = el("img");
  if (hit.icon_url) img.src = hit.icon_url;
  img.alt = "";
  img.style.cursor = "pointer";
  img.onclick = () => openProject(hit.project_id);

  const title = el("div", "title", hit.title);
  title.onclick = () => openProject(hit.project_id);

  const body = el("div", "body");
  body.append(
    title,
    el("div", "desc", hit.description),
    el("div", "meta", `by ${hit.author} | ${hit.downloads.toLocaleString()} downloads`)
  );

  const isDone = installed.has(hit.project_id);
  const installBtn = el("button", isDone ? "done" : "", isDone ? "✓ installed" : "install");
  installBtn.onclick = () => openInstallPicker(hit.project_id);

  item.dataset.id = hit.project_id;
  item.append(img, body, installBtn);
  return item;
}

function render(hits) {
  const list = $("list");
  list.replaceChildren(...hits.map(renderItem));
  $("msg").textContent = hits.length ? "" : "nothing found";
}

function markInstalled(projectId) {
  installed.add(projectId);
  const btn = document.querySelector(`.item[data-id="${projectId}"] > button`);
  if (btn) {
    btn.textContent = "✓ installed";
    btn.classList.add("done");
  }
}

// -----------------------------------------------------------------------
// Tag filters
// -----------------------------------------------------------------------

async function loadTags() {
  try {
    const tags = await api("/api/tags");
    const box = $("tags");
    box.replaceChildren();

    for (const tag of tags) {
      const name = tag.name;
      const btn = el("div", "tag", name);
      btn.onclick = () => {
        if (selectedTags.has(name)) selectedTags.delete(name);
        else selectedTags.add(name);
        btn.classList.toggle("active");
        search($("q").value.trim());
      };
      box.append(btn);
    }
  } catch {
    // фильтры не критичны, просто не покажем их при ошибке
  }
}

// -----------------------------------------------------------------------
// Search
// -----------------------------------------------------------------------

async function search(query) {
  $("msg").textContent = "searching...";
  try {
    const r = await api("/api/search", { q: query, tags: [...selectedTags].join(",") });
    if (r.error) throw new Error(r.error);
    render(r.hits || []);
  } catch (e) {
    $("list").replaceChildren();
    $("msg").textContent = `error: ${e.message}`;
  }
}

// -----------------------------------------------------------------------
// Init
// -----------------------------------------------------------------------

$("kind").textContent = kind === "mod" ? " / mods" : " / texture packs";

$("form").onsubmit = (e) => {
  e.preventDefault();
  search($("q").value.trim());
};

api("/api/info").then((i) => {
  $("info").textContent = `game: ${i.version || "any version"} | loader: ${i.loader || "none"}`;
});

loadTags();
search("");