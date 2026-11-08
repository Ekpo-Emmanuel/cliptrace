import type { BrowseArgs, BrowsePayload, ClipCard, CliptraceApi, TagCount } from "./api";

export type MockState =
  | "empty"
  | "no-results"
  | "paused"
  | "keyword-only"
  | "dark"
  | "light";

function nowMinus(ms: number): number {
  return Date.now() - ms;
}

function sampleCards(): ClipCard[] {
  return [
    {
      id: 101,
      last_seen_at: nowMinus(2 * 60_000),
      source_exe: "msedge.exe",
      window_title: "Quo — business phone",
      preview: "https://www.quo.com/pricing",
      body: "https://www.quo.com/pricing",
      kind: "link",
      tags: [],
      pinned: false,
      copy_count: 2,
    },
    {
      id: 102,
      last_seen_at: nowMinus(15 * 60_000),
      source_exe: "Code.exe",
      window_title: "store.rs",
      preview: "pub fn list_recent(offset, limit, filter)",
      body: "pub fn list_recent(\n    offset: i64,\n    limit: i64,\n    filter: &BrowseFilter,\n) -> Result<Vec<SearchHit>, StoreError> {",
      kind: "code",
      tags: [],
      pinned: false,
      copy_count: 1,
    },
    {
      id: 103,
      last_seen_at: nowMinus(40 * 60_000),
      source_exe: "notepad.exe",
      window_title: "notes",
      preview: "Wifi is on the fridge. Ask Maya if the code changed.",
      body: "Wifi is on the fridge. Ask Maya if the code changed.",
      kind: "text",
      tags: [],
      pinned: false,
      copy_count: 1,
    },
    {
      id: 104,
      last_seen_at: nowMinus(3 * 60 * 60_000),
      source_exe: "chrome.exe",
      window_title: "Payroll",
      preview: "office-vpn.internal.example",
      body: "office-vpn.internal.example",
      kind: "text",
      tags: [],
      pinned: true,
      copy_count: 8,
    },
    {
      id: 105,
      last_seen_at: nowMinus(26 * 60 * 60_000),
      source_exe: "chrome.exe",
      window_title: "RBC Online",
      preview: "client id 448219",
      body: "client id 448219",
      kind: "text",
      tags: ["RBC app"],
      pinned: false,
      copy_count: 3,
    },
    {
      id: 106,
      last_seen_at: nowMinus(5 * 60_000),
      source_exe: "chrome.exe",
      window_title: "Stripe Dashboard - API keys",
      preview: "[stripe key sk_t…]",
      body: "[stripe key sk_t…]",
      kind: "secret",
      tags: [],
      pinned: false,
      copy_count: 1,
    },
  ];
}

function readState(): MockState | null {
  const raw = new URLSearchParams(window.location.search).get("state");
  if (
    raw === "empty" ||
    raw === "no-results" ||
    raw === "paused" ||
    raw === "keyword-only" ||
    raw === "dark" ||
    raw === "light"
  ) {
    return raw;
  }
  return null;
}

function applyTheme(state: MockState | null) {
  const root = document.documentElement;
  if (state === "dark") {
    root.classList.add("dark");
    root.classList.remove("light");
  } else if (state === "light") {
    root.classList.add("light");
    root.classList.remove("dark");
  }
}

export function createMockApi(): CliptraceApi {
  const state = readState();
  applyTheme(state);
  let cards = state === "empty" || state === "no-results" ? [] : sampleCards();
  const paused = state === "paused";

  function tagsFrom(list: ClipCard[]): TagCount[] {
    const counts = new Map<string, number>();
    for (const card of list) {
      for (const tag of card.tags) {
        counts.set(tag, (counts.get(tag) ?? 0) + 1);
      }
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
        list = list.filter((c) => c.tags.some((t) => t.toLowerCase().includes(needle)));
      }
      if (args.query.trim()) {
        const q = args.query.toLowerCase();
        list = list.filter(
          (c) =>
            c.preview.toLowerCase().includes(q) ||
            c.body.toLowerCase().includes(q) ||
            c.tags.some((t) => t.toLowerCase().includes(q)),
        );
      }
      if (state === "no-results") list = [];
      return {
        cards: list.slice(args.offset, args.offset + 50),
        pinned: cards.filter((c) => c.pinned).length,
        note: state === "keyword-only" ? "keyword-only: onnxruntime.dll missing" : null,
        paused,
      };
    },
    async listTagCounts() {
      return tagsFrom(cards);
    },
    async setPinned(id, pinned) {
      const card = must(id);
      card.pinned = pinned;
      return { ...card };
    },
    async addTag(id, tag) {
      const card = must(id);
      if (!card.tags.includes(tag)) card.tags = [...card.tags, tag];
      return { ...card, tags: [...card.tags] };
    },
    async removeTag(id, tag) {
      const card = must(id);
      card.tags = card.tags.filter((t) => t !== tag);
      return { ...card, tags: [...card.tags] };
    },
    async markClipSecret(id) {
      const card = must(id);
      card.kind = "secret";
      card.preview = "[secret token …]";
      card.body = "[secret token …]";
      return { ...card };
    },
    async deleteClip(id) {
      cards = cards.filter((c) => c.id !== id);
    },
    async copyClip(_id) {
      void _id;
    },
    async hidePopup() {},
    async captureStatus() {
      return paused;
    },
  };
}
