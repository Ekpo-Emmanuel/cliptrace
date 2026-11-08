const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

const q = document.getElementById("q");
const list = document.getElementById("list");
const noteEl = document.getElementById("note");
const stubId = document.getElementById("stub-id");

let hits = [];
let selected = 0;
let timer = 0;
let blurTimer = 0;

function hide() {
  invoke("hide_popup");
}

function render() {
  list.replaceChildren();
  hits.forEach((hit, i) => {
    const li = document.createElement("li");
    li.setAttribute("role", "option");
    li.setAttribute("aria-selected", i === selected ? "true" : "false");
    const preview = document.createElement("div");
    preview.className = "preview";
    preview.textContent = hit.preview || "";
    const time = document.createElement("div");
    time.className = "meta";
    time.textContent = `${hit.abs_time} · ${hit.rel_time}`;
    const exe = document.createElement("div");
    exe.className = "meta exe";
    exe.textContent = hit.source_exe || "";
    li.append(preview, time, exe);
    li.addEventListener("mousedown", (e) => {
      e.preventDefault();
      selected = i;
      choose();
    });
    list.append(li);
  });
  const cur = hits[selected];
  stubId.textContent = cur ? String(cur.id).padStart(4, "0") : "—";
}

async function runSearch() {
  const query = q.value;
  try {
    const payload = await invoke("search_clips", { query });
    hits = payload.hits || [];
    selected = 0;
    if (payload.note) {
      noteEl.hidden = false;
      noteEl.textContent = payload.note;
    } else {
      noteEl.hidden = true;
      noteEl.textContent = "";
    }
    render();
  } catch {
    hits = [];
    render();
  }
}

function scheduleSearch() {
  clearTimeout(timer);
  timer = setTimeout(runSearch, 150);
}

async function choose() {
  const hit = hits[selected];
  if (!hit) return;
  await invoke("copy_clip", { id: hit.id });
}

q.addEventListener("input", scheduleSearch);

q.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    if (hits.length) selected = Math.min(hits.length - 1, selected + 1);
    render();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    if (hits.length) selected = Math.max(0, selected - 1);
    render();
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose();
  } else if (e.key === "Escape") {
    e.preventDefault();
    hide();
  }
});

window.addEventListener("blur", () => {
  clearTimeout(blurTimer);
  blurTimer = setTimeout(hide, 80);
});

window.addEventListener("focus", () => clearTimeout(blurTimer));

window.cliptraceOnOpen = () => {
  q.value = "";
  hits = [];
  selected = 0;
  noteEl.hidden = true;
  render();
  q.focus();
  runSearch();
};

q.focus();
