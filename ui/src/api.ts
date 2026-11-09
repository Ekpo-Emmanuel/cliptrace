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

export type AppStatus = {
  paused: boolean;
  keyword_only: boolean;
};

export type CliptraceApi = {
  browseClips: (args: BrowseArgs) => Promise<BrowsePayload>;
  listTagCounts: () => Promise<TagCount[]>;
  setPinned: (id: number, pinned: boolean) => Promise<ClipCard>;
  addTag: (id: number, tag: string) => Promise<ClipCard>;
  removeTag: (id: number, tag: string) => Promise<ClipCard>;
  markClipSecret: (id: number) => Promise<ClipCard>;
  deleteClip: (id: number) => Promise<void>;
  stageDelete: (id: number) => Promise<void>;
  unstageDelete: (id: number) => Promise<void>;
  clipText: (id: number) => Promise<string>;
  copyClip: (id: number) => Promise<void>;
  hidePopup: () => Promise<void>;
  startDrag: () => Promise<void>;
  setMode: (compact: boolean) => Promise<void>;
  getSettings: () => Promise<Settings>;
  updateSettings: (patch: SettingsPatch) => Promise<Settings>;
  status: () => Promise<AppStatus>;
  onClipsChanged: (cb: () => void) => Promise<() => void>;
  onStatusChanged: (cb: (s: AppStatus) => void) => Promise<() => void>;
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
    listTagCounts: () => invoke<TagCount[]>("list_tags"),
    setPinned: (id, pinned) => invoke<ClipCard>("set_pinned", { id, pinned }),
    addTag: (id, tag) => invoke<ClipCard>("add_tag", { id, tag }),
    removeTag: (id, tag) => invoke<ClipCard>("remove_tag", { id, tag }),
    markClipSecret: (id) => invoke<ClipCard>("mark_clip_secret", { id }),
    deleteClip: (id) => invoke("delete_clip", { id }),
    stageDelete: (id) => invoke("stage_delete", { id }),
    unstageDelete: (id) => invoke("unstage_delete", { id }),
    clipText: (id) => invoke<string>("clip_text", { id }),
    copyClip: (id) => invoke("copy_clip", { id }),
    hidePopup: () => invoke("hide_popup"),
    setMode: (compact) => invoke("set_popup_mode", { compact }),
    getSettings: () => invoke<Settings>("get_settings"),
    updateSettings: (patch) => invoke<Settings>("update_settings", { patch }),
    startDrag: async () => {
      await invoke("begin_move");
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    },
    status: () => invoke<AppStatus>("status"),
    onClipsChanged: async (cb) => {
      const { listen } = await import("@tauri-apps/api/event");
      const un = await listen("clips-changed", () => cb());
      return () => un();
    },
    onStatusChanged: async (cb) => {
      const { listen } = await import("@tauri-apps/api/event");
      const un = await listen<AppStatus>("status-changed", (e) => cb(e.payload));
      return () => un();
    },
  };
}

let ready: Promise<CliptraceApi> | null = null;

export function tauriAvailable(): boolean {
  return isTauri();
}

export function getApi(): Promise<CliptraceApi> {
  if (!ready) {
    ready = (async () => {
      if (isTauri()) {
        const { invoke } = await import("@tauri-apps/api/core");
        return createTauriApi(invoke as Invoke);
      }
      if (import.meta.env.DEV) {
        const { createMockApi } = await import("./mock");
        return createMockApi();
      }
      throw new Error("Tauri IPC is not available");
    })();
  }
  return ready;
}

export type Settings = {
  paused: boolean;
  keyword_only: boolean;
  hotkey: string;
  secret_ttl_hours: number;
  retention_days: number;
  blocklist: string[];
  autostart: boolean;
  welcomed: boolean;
  data_dir: string;
  version: string;
};

export type SettingsPatch = Partial<
  Pick<
    Settings,
    | "paused"
    | "autostart"
    | "welcomed"
    | "secret_ttl_hours"
    | "retention_days"
    | "blocklist"
  >
>;
