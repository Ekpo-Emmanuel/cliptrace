import type {
  BrowseArgs,
  BrowsePayload,
  ClipCard,
  CliptraceApi,
  Settings,
  TagCount,
} from "./api";

export type MockState =
  | "empty"
  | "no-results"
  | "paused"
  | "keyword-only"
  | "dark"
  | "light"
  | "nopins";

const TAGS = ["work", "dev", "meeting", "home", "finance", "travel", "docs", "design"];

function ago(ms: number): number {
  return Date.now() - ms;
}

function seedCard(id: number): ClipCard {
  const hour = 3_600_000;
  const day = 86_400_000;
  const slot = id - 200;
  const kinds: ClipCard["kind"][] = [
    "text",
    "link",
    "code",
    "text",
    "link",
    "code",
    "text",
  ];
  const kind = kinds[slot % kinds.length];
  const apps = [
    ["msedge.exe", "Inbox — Outlook"],
    ["Code.exe", "cliptrace / store.rs"],
    ["chrome.exe", "GitHub"],
    ["Slack.exe", "eng-help"],
    ["WindowsTerminal.exe", "pwsh"],
    ["notepad.exe", "scratch"],
    ["EXCEL.EXE", "Q3 forecast"],
    ["Figma.exe", "Popup — Frame 12"],
  ] as const;
  const [exe, title] = apps[slot % apps.length];
  const pinned = false;
  const tag = slot % 9 === 0 ? [TAGS[slot % TAGS.length]] : [];
  const bodies: Record<string, string> = {
    text: [
      "Dinner reservation is at 7:30 under the name Morgan.",
      "Follow up with Maya after the 2pm standup about the invoice.",
      "Kids pickup is 3:40 at the side door, not the main lot.",
      "The printer on 4 is jam-prone. Use the one by finance.",
      "Ship date slipped to Thursday. Tell the client before noon.",
    ][slot % 5],
    link: [
      "https://github.com/acme/storefront/pull/418",
      "https://docs.acme.dev/api/webhooks/signatures",
      "https://docs.google.com/spreadsheets/d/budget-q4",
      "https://stackoverflow.com/questions/7281/rust-fts5",
      "https://learn.microsoft.com/windows/win32/dataxchg/clipboard",
    ][slot % 5],
    code: [
      "const hits = await api.browseClips({ query, tab, tag, offset: 0 });",
      "UPDATE clips SET pinned_at = ?1 WHERE id = ?2;",
      "Get-Process cliptrace-app | Select-Object Id, WorkingSet64",
      "fn relTime(ms: i64, now: i64) -> String { /* injected now */ }",
      "git log --oneline -20",
    ][slot % 5],
  };
  const body = bodies[kind] ?? bodies.text;
  return {
    id,
    last_seen_at: ago(hour + (slot % 40) * hour + Math.floor(slot / 12) * day),
    source_exe: exe,
    window_title: title,
    preview: body.split("\n")[0].slice(0, 80),
    body,
    kind,
    tags: tag,
    pinned,
    copy_count: 1 + (slot % 6),
  };
}

function sampleCards(): ClipCard[] {
  const featured: ClipCard[] = [
    {
      id: 101,
      last_seen_at: ago(3 * 60_000),
      source_exe: "outlook.exe",
      window_title: "Re: Q4 contract — Outlook",
      preview: "Hi Maya, thanks for sending the contract over. Everything looks good on my side,",
      body: "Hi Maya, thanks for sending the contract over. Everything looks good on my side, and I'll return the signed copy by Friday. Best, Alex",
      kind: "text",
      tags: ["work"],
      pinned: true,
      copy_count: 1,
    },
    {
      id: 102,
      last_seen_at: ago(9 * 60_000),
      source_exe: "chrome.exe",
      window_title: "Webhook signatures — Acme API",
      preview: "https://docs.acme.dev/api/webhooks/signatures",
      body: "https://docs.acme.dev/api/webhooks/signatures",
      kind: "link",
      tags: ["dev"],
      pinned: false,
      copy_count: 2,
    },
    {
      id: 103,
      last_seen_at: ago(16 * 60_000),
      source_exe: "chrome.exe",
      window_title: "API keys — Dashboard",
      preview: "[API key live_k…]",
      body: "[API key live_k…]",
      kind: "secret",
      tags: [],
      pinned: false,
      copy_count: 1,
    },
    {
      id: 104,
      last_seen_at: ago(24 * 60_000),
      source_exe: "Slack.exe",
      window_title: "design-review",
      preview: "Standup moved to 10:15. Bring the Q4 roadmap draft.",
      body: "Standup moved to 10:15. Bring the Q4 roadmap draft.",
      kind: "text",
      tags: ["meeting"],
      pinned: true,
      copy_count: 2,
    },
    {
      id: 105,
      last_seen_at: ago(38 * 60_000),
      source_exe: "Code.exe",
      window_title: "cart.ts — storefront",
      preview: "export function cartTotal(items: Item[]): number {",
      body: "export function cartTotal(items: Item[]): number {\n  return items.reduce((sum, i) => sum + i.price * i.qty, 0);\n}",
      kind: "code",
      tags: ["dev"],
      pinned: false,
      copy_count: 1,
    },
    {
      id: 106,
      last_seen_at: ago(52 * 60_000),
      source_exe: "chrome.exe",
      window_title: "Track your parcel",
      preview: "Tracking number 7421 0985 3367",
      body: "Tracking number 7421 0985 3367",
      kind: "text",
      tags: [],
      pinned: false,
      copy_count: 1,
    },
    {
      id: 107,
      last_seen_at: ago(180 * 60_000),
      source_exe: "notepad.exe",
      window_title: "Addresses",
      preview: "221B Baker Street, London NW1 6XE",
      body: "221B Baker Street, London NW1 6XE",
      kind: "text",
      tags: ["home"],
      pinned: true,
      copy_count: 6,
    },
    {
      id: 108,
      last_seen_at: ago(1560 * 60_000),
      source_exe: "winword.exe",
      window_title: "Lease renewal.docx",
      preview: "Renewal notice must be given 60 days before the end of term.",
      body: "Renewal notice must be given 60 days before the end of term.",
      kind: "text",
      tags: ["finance"],
      pinned: false,
      copy_count: 1,
    },
    {
      id: 109,
      last_seen_at: ago(1680 * 60_000),
      source_exe: "outlook.exe",
      window_title: "Your flight confirmation — Outlook",
      preview: "Flight BA 117 departs at 21:40 from Terminal 5.",
      body: "Flight BA 117 departs at 21:40 from Terminal 5.",
      kind: "text",
      tags: ["travel"],
      pinned: false,
      copy_count: 1,
    },
  ];
  const extra: ClipCard[] = [];
  for (let i = 0; i < 100; i += 1) extra.push(seedCard(200 + i));
  return [...featured, ...extra].sort(
    (a, b) => b.last_seen_at - a.last_seen_at,
  );
}

function readState(): MockState | null {
  const raw = new URLSearchParams(window.location.search).get("state");
  const allowed: MockState[] = [
    "empty",
    "no-results",
    "paused",
    "keyword-only",
    "dark",
    "light",
    "nopins",
  ];
  return allowed.includes(raw as MockState) ? (raw as MockState) : null;
}

function applyTheme(state: MockState | null) {
  const root = document.documentElement;
  const preferDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const dark = state === "dark" || (state !== "light" && preferDark);
  root.classList.toggle("dark", dark);
  root.classList.toggle("light", !dark);
}

export function createMockApi(): CliptraceApi {
  let settings: Settings = {
    paused: false,
    keyword_only: false,
    hotkey: "Ctrl+Alt+V",
    secret_ttl_hours: 24,
    retention_days: 90,
    blocklist: ["keepass.exe", "1password.exe"],
    autostart: false,
    // Browser dev: open /?welcome to preview the first-run screen.
    welcomed: !new URLSearchParams(window.location.search).has("welcome"),
    data_dir: "C:\Users\you\AppData\Local\Cliptrace",
    version: "0.1.0",
  };
  const state = readState();
  applyTheme(state);
  let cards = state === "empty" || state === "no-results" ? [] : sampleCards();
  if (state === "nopins") {
    cards = cards.map((c) => ({ ...c, pinned: false }));
  }
  const paused = state === "paused";

  function tagsFrom(list: ClipCard[]): TagCount[] {
    const counts = new Map<string, number>();
    for (const card of list) {
      for (const tag of card.tags) counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
    return [...counts.entries()].map(([name, count]) => ({ name, count }));
  }

  function must(id: number): ClipCard {
    const card = cards.find((c) => c.id === id);
    if (!card) throw new Error(`clip ${id} not found`);
    return card;
  }

  return {
    async browseClips(args: BrowseArgs): Promise<BrowsePayload> {
      let list = cards.slice();
      if (args.tab === "pinned") list = list.filter((c) => c.pinned);
      if (args.tag) {
        const needle = args.tag.toLowerCase();
        list = list.filter((c) =>
          c.tags.some((t) => t.toLowerCase().includes(needle)),
        );
      }
      if (args.query.trim()) {
        const q = args.query.toLowerCase();
        list = list.filter(
          (c) =>
            c.preview.toLowerCase().includes(q) ||
            c.body.toLowerCase().includes(q) ||
            c.tags.some((t) => t.toLowerCase().includes(q)) ||
            c.source_exe.toLowerCase().includes(q),
        );
      }
      if (state === "no-results") list = [];
      return {
        cards: list.slice(args.offset, args.offset + 50),
        pinned: cards.filter((c) => c.pinned).length,
        total: cards.length,
        note:
          state === "keyword-only"
            ? "keyword-only: onnxruntime.dll missing"
            : null,
        paused,
      };
    },
    async listTagCounts() {
      return tagsFrom(cards);
    },
    async setPinned(id, pinned) {
      const card = must(id);
      card.pinned = pinned;
      return { ...card, tags: [...card.tags] };
    },
    async addTag(id, tag) {
      const card = must(id);
      const name = tag.trim();
      if (
        name &&
        !card.tags.some((t) => t.toLowerCase() === name.toLowerCase()) &&
        card.tags.length < 10
      ) {
        card.tags = [...card.tags, name];
      }
      return { ...card, tags: [...card.tags] };
    },
    async removeTag(id, tag) {
      const card = must(id);
      card.tags = card.tags.filter(
        (t) => t.toLowerCase() !== tag.toLowerCase(),
      );
      return { ...card, tags: [...card.tags] };
    },
    async markClipSecret(id) {
      const card = must(id);
      card.kind = "secret";
      card.preview = "[secret token …]";
      card.body = "[secret token …]";
      return { ...card, tags: [...card.tags] };
    },
    async deleteClip(id) {
      cards = cards.filter((c) => c.id !== id);
    },
    async copyClip(_id) {
      void _id;
    },
    async hidePopup() {},
    async startDrag() {},
    async setMode() {},
    async getSettings() {
      return { ...settings, blocklist: [...settings.blocklist] };
    },
    async updateSettings(patch) {
      settings = { ...settings, ...patch };
      return { ...settings, blocklist: [...settings.blocklist] };
    },
    async stageDelete(_id) {
      void _id;
    },
    async unstageDelete(_id) {
      void _id;
    },
    async clipText(id) {
      const card = must(id);
      if (card.kind === "secret") throw new Error(`clip ${id} is a secret`);
      return card.body;
    },
    async status() {
      return {
        paused,
        keyword_only: state === "keyword-only",
      };
    },
    async onClipsChanged() {
      return () => {};
    },
    async onStatusChanged() {
      return () => {};
    },
  };
}
