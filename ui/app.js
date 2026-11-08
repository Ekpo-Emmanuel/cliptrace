const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

const q = document.getElementById("q");
const list = document.getElementById("list");
const noteEl = document.getElementById("note");
const emptyEl = document.getElementById("empty");
const preview = document.getElementById("preview");
const suggest = document.getElementById("suggest");
const tagGrid = document.getElementById("tag-grid");
const pinCount = document.getElementById("pin-count");
const pausedEl = document.getElementById("paused");
const toastEl = document.getElementById("toast");
const confirmEl = document.getElementById("confirm");
const confirmText = document.getElementById("confirm-text");
const confirmOk = document.getElementById("confirm-ok");
const confirmCancel = document.getElementById("confirm-cancel");

const tabs = {
  recent: document.getElementById("tab-recent"),
  pinned: document.getElementById("tab-pinned"),
  tags: document.getElementById("tab-tags"),
};

let tab = "recent";
let cards = [];
let selected = 0;
let offset = 0;
let loading = false;
let done = false;
let timer = 0;
let blurTimer = 0;
let toastTimer = 0;
let deleteTimer = 0;
let pendingDelete = null;
let allTags = [];
let suggestItems = [];
let suggestIndex = 0;
let activeTag = "";
let tagFocus = false;

const pinSvg = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 3h8l-1 7 3 3v2h-5v6l-1 1-1-1v-6H6v-2l3-3z" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>`;
const tagSvg = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12l8-8h6v6l-8 8z" fill="none" stroke="currentColor" stroke-width="1.6"/><circle cx="15.5" cy="8.5" r="1" fill="currentColor"/></svg>`;

function hide() {
  invoke("hide_popup");
}

function setPaused(on) {
  pausedEl.hidden = !on;
}

function relTime(ms) {
  const delta = Date.now() - ms;
  const min = Math.round(delta / 60000);
  if (min < 1) return "just now";
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  return `${day}d ago`;
}

function dayLabel(ms) {
  const d = new Date(ms);
  const now = new Date();
  const start = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = (start(now) - start(d)) / 86400000;
  if (diff === 0) return "Today";
  if (diff === 1) return "Yesterday";
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

function showNote(text) {
  if (text) {
    noteEl.hidden = false;
    noteEl.textContent = text;
  } else {
    noteEl.hidden = true;
    noteEl.textContent = "";
  }
}

function emptyCopy() {
  if (q.value.trim()) return "No results. Try wifi password last week.";
  if (tab === "pinned") return "Pin things you reuse with Ctrl+P.";
  if (tab === "tags" && !activeTag) return "";
  return "Copy something and it’ll show up here.";
}

function renderList() {
  list.replaceChildren();
  const showList = !(tab === "tags" && !activeTag);
  list.hidden = !showList;
  tagGrid.hidden = tab !== "tags" || !!activeTag;
  if (!showList) {
    emptyEl.hidden = true;
    preview.replaceChildren();
    return;
  }
  if (!cards.length) {
    emptyEl.hidden = false;
    emptyEl.textContent = emptyCopy();
    preview.replaceChildren();
    return;
  }
  emptyEl.hidden = true;
  let lastDay = "";
  cards.forEach((card, i) => {
    const day = dayLabel(card.last_seen_at);
    if (day !== lastDay) {
      lastDay = day;
      const label = document.createElement("li");
      label.className = "day";
      label.textContent = day;
      list.append(label);
    }
    const li = document.createElement("li");
    const row = document.createElement("div");
    row.className = "row";
    row.setAttribute("role", "option");
    row.id = `clip-${card.id}`;
    row.setAttribute("aria-selected", i === selected ? "true" : "false");
    const stamp = document.createElement("span");
    stamp.className = "stamp";
    stamp.textContent = card.kind || "text";
    const line = document.createElement("div");
    line.className = "line";
    line.textContent = card.preview || card.body || "";
    const meta = document.createElement("div");
    meta.className = "meta";
    const exe = document.createElement("span");
    exe.textContent = card.source_exe || "unknown";
    const when = document.createElement("span");
    when.textContent = relTime(card.last_seen_at);
    meta.append(exe, when);
    const tags = card.tags || [];
    tags.slice(0, 2).forEach((name) => {
      const chip = document.createElement("span");
      chip.className = "chip";
      chip.textContent = name;
      meta.append(chip);
    });
    if (tags.length > 2) {
      const more = document.createElement("span");
      more.className = "chip";
      more.textContent = `+${tags.length - 2}`;
      meta.append(more);
    }
    if (card.pinned) {
      const pin = document.createElement("span");
      pin.className = "pin-on";
      pin.innerHTML = pinSvg;
      pin.querySelector("svg").setAttribute("width", "12");
      pin.querySelector("svg").setAttribute("height", "12");
      meta.append(pin);
    }
    const actions = document.createElement("div");
    actions.className = "hover-actions";
    const pinBtn = iconButton(pinSvg, card.pinned ? "Unpin" : "Pin", () => togglePin(card));
    const tagBtn = iconButton(tagSvg, "Add tag", () => {
      selected = i;
      renderList();
      const input = document.getElementById("add-tag");
      if (input) input.focus();
    });
    actions.append(pinBtn, tagBtn);
    row.append(stamp, line, meta, actions);
    row.addEventListener("mousedown", (e) => {
      if (e.target.closest("button")) return;
      e.preventDefault();
      selected = i;
      renderList();
      q.focus();
    });
    li.append(row);
    list.append(li);
  });
  const current = list.querySelector('[aria-selected="true"]');
  if (current) current.scrollIntoView({ block: "nearest" });
  renderPreview();
}

function iconButton(svg, label, onClick) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "icon-btn";
  btn.innerHTML = svg;
  btn.setAttribute("aria-label", label);
  btn.addEventListener("mousedown", (e) => e.preventDefault());
  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    onClick();
  });
  return btn;
}

function renderPreview() {
  const card = cards[selected];
  preview.replaceChildren();
  if (!card) return;
  const title = document.createElement("h2");
  title.textContent = card.kind === "secret" ? "Secret" : "Clip";
  const facts = document.createElement("p");
  facts.className = "facts";
  const when = new Date(card.last_seen_at).toLocaleString();
  facts.textContent = `${card.source_exe || "unknown"} · ${card.window_title || "no window"} · ${when} · copied ${card.copy_count || 1}`;
  const body = document.createElement("pre");
  body.className = card.kind === "code" ? "body code" : "body";
  body.textContent = card.body || card.preview || "";
  const tags = document.createElement("div");
  tags.className = "tags";
  (card.tags || []).forEach((name) => {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "chip";
    chip.textContent = name;
    const x = document.createElement("span");
    x.className = "x";
    x.textContent = "×";
    x.setAttribute("aria-hidden", "true");
    chip.append(x);
    chip.setAttribute("aria-label", `Remove tag ${name}`);
    chip.addEventListener("click", () => removeTag(card, name));
    tags.append(chip);
  });
  const add = document.createElement("input");
  add.id = "add-tag";
  add.className = "add-tag";
  add.placeholder = "Add tag…";
  add.setAttribute("aria-label", "Add tag");
  add.autocomplete = "off";
  add.addEventListener("keydown", (e) => onAddTagKey(e, card, add));
  add.addEventListener("focus", () => { tagFocus = true; });
  add.addEventListener("blur", () => { tagFocus = false; });
  const hint = document.createElement("p");
  hint.className = "hint";
  hint.textContent = "Tags are stored as plain text. Don’t put passwords in them.";
  const actions = document.createElement("div");
  actions.className = "actions";
  actions.append(
    action("Copy", "primary", () => choose()),
    action(card.pinned ? "Unpin" : "Pin", "ghost", () => togglePin(card)),
    action("Mark as secret", "ghost", () => askSecret(card)),
    action("Delete", "ghost danger", () => askDelete(card)),
  );
  preview.append(title, facts, body, tags, add, hint, actions);
}

function action(label, cls, onClick) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = cls;
  btn.textContent = label;
  btn.addEventListener("click", onClick);
  return btn;
}

function renderTagGrid() {
  tagGrid.replaceChildren();
  if (!allTags.length) {
    const p = document.createElement("p");
    p.className = "empty";
    p.textContent = "No tags yet. Add one from a clip with Ctrl+T.";
    tagGrid.append(p);
    return;
  }
  allTags.forEach((tag) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = `${tag.name} ${tag.count}`;
    btn.setAttribute("aria-pressed", activeTag === tag.name ? "true" : "false");
    btn.addEventListener("click", () => {
      activeTag = tag.name;
      offset = 0;
      done = false;
      load(true);
    });
    tagGrid.append(btn);
  });
}

async function load(reset) {
  if (loading) return;
  if (!reset && done) return;
  loading = true;
  if (reset) {
    offset = 0;
    done = false;
  }
  try {
    const payload = await invoke("browse_clips", {
      query: q.value,
      tab: tab === "tags" ? "recent" : tab,
      tag: activeTag,
      offset,
    });
    const next = payload.cards || [];
    cards = reset ? next : cards.concat(next);
    if (reset) selected = 0;
    pinCount.textContent = payload.pinned ? String(payload.pinned) : "";
    setPaused(!!payload.paused);
    showNote(payload.note || "");
    if (q.value.trim()) done = true;
    else if (next.length < 50) done = true;
    else offset += next.length;
    renderList();
  } catch {
    if (reset) cards = [];
    renderList();
  } finally {
    loading = false;
  }
}

function schedule() {
  clearTimeout(timer);
  timer = setTimeout(() => {
    updateSuggest();
    load(true);
  }, 150);
}

async function refreshTags() {
  try {
    allTags = await invoke("list_tag_counts");
  } catch {
    allTags = [];
  }
  renderTagGrid();
}

function updateSuggest() {
  const text = q.value;
  const match = text.match(/(?:^|\s)#([^\s]*)$/);
  if (!match) {
    suggest.hidden = true;
    q.setAttribute("aria-expanded", "false");
    suggestItems = [];
    return;
  }
  const prefix = match[1].toLowerCase();
  suggestItems = allTags.filter((t) => t.name.toLowerCase().includes(prefix)).slice(0, 8);
  suggest.replaceChildren();
  suggestItems.forEach((tag, i) => {
    const li = document.createElement("li");
    li.setAttribute("role", "option");
    li.id = `sug-${i}`;
    li.setAttribute("aria-selected", i === 0 ? "true" : "false");
    li.textContent = tag.name;
    li.addEventListener("mousedown", (e) => {
      e.preventDefault();
      applySuggest(tag.name);
    });
    suggest.append(li);
  });
  suggestIndex = 0;
  suggest.hidden = suggestItems.length === 0;
  q.setAttribute("aria-expanded", suggest.hidden ? "false" : "true");
}

function applySuggest(name) {
  q.value = q.value.replace(/(?:^|\s)#([^\s]*)$/, (all) => {
    const space = all.startsWith(" ") || all.startsWith("#") ? (all.startsWith(" ") ? " " : "") : "";
    return `${space}#${name} `;
  });
  suggest.hidden = true;
  q.focus();
  load(true);
}

function move(delta) {
  if (!cards.length) return;
  selected = Math.max(0, Math.min(cards.length - 1, selected + delta));
  renderList();
  if (selected > cards.length - 8) load(false);
}

async function choose() {
  const card = cards[selected];
  if (!card) return;
  await invoke("copy_clip", { id: card.id });
  showToast("Copied");
  setTimeout(hide, 280);
}

async function togglePin(card) {
  const next = await invoke("set_pinned", { id: card.id, pinned: !card.pinned });
  replaceCard(next);
  pinCount.textContent = cards.filter((c) => c.pinned).length ? String(cards.filter((c) => c.pinned).length) : pinCount.textContent;
  if (tab === "pinned" && !next.pinned) {
    cards = cards.filter((c) => c.id !== next.id);
    selected = Math.max(0, Math.min(selected, cards.length - 1));
  }
  renderList();
  load(true);
}

async function removeTag(card, name) {
  const next = await invoke("remove_tag", { id: card.id, tag: name });
  replaceCard(next);
  await refreshTags();
  renderList();
}

async function addTag(card, raw) {
  const name = raw.trim();
  if (!name) return;
  const next = await invoke("add_tag", { id: card.id, tag: name });
  replaceCard(next);
  await refreshTags();
  renderList();
}

function onAddTagKey(e, card, input) {
  if (e.key === "Enter") {
    e.preventDefault();
    e.stopPropagation();
    const value = input.value;
    input.value = "";
    addTag(card, value);
  } else if (e.key === "Backspace" && !input.value && (card.tags || []).length) {
    e.preventDefault();
    removeTag(card, card.tags[card.tags.length - 1]);
  } else if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    q.focus();
  }
}

function replaceCard(next) {
  const i = cards.findIndex((c) => c.id === next.id);
  if (i >= 0) cards[i] = next;
}

function askSecret(card) {
  if (card.kind === "secret") return;
  confirmText.textContent = "Mark as secret is one-way. The plaintext is encrypted and removed from the database.";
  confirmEl.hidden = false;
  confirmOk.onclick = async () => {
    confirmEl.hidden = true;
    const next = await invoke("mark_clip_secret", { id: card.id });
    replaceCard(next);
    renderList();
  };
  confirmCancel.onclick = () => {
    confirmEl.hidden = true;
    q.focus();
  };
  confirmCancel.focus();
}

function askDelete(card) {
  if (pendingDelete) commitDelete();
  const saved = { card, index: selected };
  cards = cards.filter((c) => c.id !== card.id);
  selected = Math.max(0, Math.min(selected, cards.length - 1));
  renderList();
  pendingDelete = saved;
  showToast("Deleted", "Undo", () => {
    clearTimeout(deleteTimer);
    cards.splice(saved.index, 0, saved.card);
    selected = saved.index;
    pendingDelete = null;
    renderList();
  });
  deleteTimer = setTimeout(commitDelete, 5000);
}

function commitDelete() {
  if (!pendingDelete) return;
  const id = pendingDelete.card.id;
  pendingDelete = null;
  invoke("delete_clip", { id }).then(() => load(true));
}

function showToast(text, actionLabel, onAction) {
  clearTimeout(toastTimer);
  toastEl.replaceChildren();
  toastEl.hidden = false;
  toastEl.append(document.createTextNode(text));
  if (actionLabel) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = actionLabel;
    btn.addEventListener("click", () => {
      toastEl.hidden = true;
      onAction();
    });
    toastEl.append(btn);
  }
  toastTimer = setTimeout(() => {
    toastEl.hidden = true;
  }, actionLabel ? 5000 : 900);
}

function setTab(next) {
  tab = next;
  activeTag = next === "tags" ? activeTag : "";
  Object.entries(tabs).forEach(([name, el]) => {
    el.setAttribute("aria-selected", name === next ? "true" : "false");
  });
  if (next === "tags") refreshTags();
  load(true);
}

list.addEventListener("scroll", () => {
  if (list.scrollTop + list.clientHeight > list.scrollHeight - 40) load(false);
});

q.addEventListener("input", schedule);

document.addEventListener("keydown", (e) => {
  if (!confirmEl.hidden && e.key === "Escape") {
    e.preventDefault();
    confirmEl.hidden = true;
    return;
  }
  if (e.key === "Escape") {
    e.preventDefault();
    if (!suggest.hidden) {
      suggest.hidden = true;
      return;
    }
    hide();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key === "1") {
    e.preventDefault();
    setTab("recent");
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key === "2") {
    e.preventDefault();
    setTab("pinned");
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key === "3") {
    e.preventDefault();
    setTab("tags");
    return;
  }
  if (tagFocus) return;
  if (!suggest.hidden && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
    e.preventDefault();
    suggestIndex = Math.max(0, Math.min(suggestItems.length - 1, suggestIndex + (e.key === "ArrowDown" ? 1 : -1)));
    [...suggest.children].forEach((li, i) => li.setAttribute("aria-selected", i === suggestIndex ? "true" : "false"));
    return;
  }
  if (!suggest.hidden && e.key === "Enter") {
    e.preventDefault();
    const tag = suggestItems[suggestIndex];
    if (tag) applySuggest(tag.name);
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "p") {
    e.preventDefault();
    if (cards[selected]) togglePin(cards[selected]);
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "t") {
    e.preventDefault();
    const input = document.getElementById("add-tag");
    if (input) input.focus();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "l") {
    e.preventDefault();
    if (cards[selected]) askSecret(cards[selected]);
    return;
  }
  if (e.key === "Delete" && document.activeElement === q) {
    e.preventDefault();
    if (cards[selected]) askDelete(cards[selected]);
    return;
  }
  if (document.activeElement !== q) return;
  if (e.key === "ArrowDown") {
    e.preventDefault();
    move(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    move(-1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose();
  }
});

tabs.recent.addEventListener("click", () => setTab("recent"));
tabs.pinned.addEventListener("click", () => setTab("pinned"));
tabs.tags.addEventListener("click", () => setTab("tags"));

window.addEventListener("blur", () => {
  clearTimeout(blurTimer);
  blurTimer = setTimeout(hide, 120);
});
window.addEventListener("focus", () => clearTimeout(blurTimer));

window.cliptraceOnOpen = (cache) => {
  q.value = "";
  tab = "recent";
  activeTag = "";
  selected = 0;
  offset = 0;
  done = false;
  Object.entries(tabs).forEach(([name, el]) => {
    el.setAttribute("aria-selected", name === "recent" ? "true" : "false");
  });
  cards = (cache && cache.cards) || [];
  setPaused(!!(cache && cache.paused));
  showNote("");
  suggest.hidden = true;
  confirmEl.hidden = true;
  tagGrid.hidden = true;
  renderList();
  q.focus();
  refreshTags();
  load(true);
};

q.focus();
