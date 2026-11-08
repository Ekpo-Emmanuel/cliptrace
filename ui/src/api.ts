export type ClipKind = "link" | "code" | "text" | "secret" | string;

export type ClipCard = {
  id: number;
  last_seen_at: number;
  source_exe: string;
  window_title: string;
  preview: string;
  body: string;
  kind: ClipKind;
  tags: string[];
  pinned: boolean;
  copy_count: number;
};

export type BrowsePayload = {
  cards: ClipCard[];
  pinned: number;
  total: number;
  note: string | null;
  paused: boolean;
};

export type TagCount = {
  name: string;
  count: number;
};

export type BrowseArgs = {
  query: string;
  tab: string;
  tag: string;
  offset: number;
};

export type CliptraceApi = {
  browseClips: (args: BrowseArgs) => Promise<BrowsePayload>;
  listTagCounts: () => Promise<TagCount[]>;
  setPinned: (id: number, pinned: boolean) => Promise<ClipCard>;
  addTag: (id: number, tag: string) => Promise<ClipCard>;
  removeTag: (id: number, tag: string) => Promise<ClipCard>;
  markClipSecret: (id: number) => Promise<ClipCard>;
  deleteClip: (id: number) => Promise<void>;
  copyClip: (id: number) => Promise<void>;
  hidePopup: () => Promise<void>;
  captureStatus: () => Promise<boolean>;
};

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriApi(invoke: Invoke): CliptraceApi {
  return {
    browseClips: (args) =>
      invoke<BrowsePayload>("browse_clips", {
        query: args.query,
        tab: args.tab,
        tag: args.tag,
        offset: args.offset,
      }),
    listTagCounts: () => invoke<TagCount[]>("list_tag_counts"),
    setPinned: (id, pinned) => invoke<ClipCard>("set_pinned", { id, pinned }),
    addTag: (id, tag) => invoke<ClipCard>("add_tag", { id, tag }),
    removeTag: (id, tag) => invoke<ClipCard>("remove_tag", { id, tag }),
    markClipSecret: (id) => invoke<ClipCard>("mark_clip_secret", { id }),
    deleteClip: (id) => invoke("delete_clip", { id }),
    copyClip: (id) => invoke("copy_clip", { id }),
    hidePopup: () => invoke("hide_popup"),
    captureStatus: () => invoke<boolean>("capture_status"),
  };
}

let ready: Promise<CliptraceApi> | null = null;

export function tauriAvailable(): boolean {
  return isTauri();
}

export function getApi(): Promise<CliptraceApi> {
  if (!ready) {
    ready = (async () => {
      const { createMockApi } = await import("./mock");
      return createMockApi();
    })();
  }
  return ready;
}
