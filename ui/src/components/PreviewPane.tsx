import type { ClipCard, TagCount } from "../api";
import { IconCopy, IconLock, IconPin, IconTrash } from "../icons";
import { kindMark } from "../lib/kind";
import { relTime } from "../lib/time";
import { ClipContent } from "./ClipContent";
import { TagEditor } from "./TagEditor";

type Props = {
  card: ClipCard | undefined;
  position: number;
  allTags: TagCount[];
  copied: boolean;
  onCopy: () => void;
  onPin: () => void;
  onSecret: () => void;
  onDelete: () => void;
  onAddTag: (tag: string) => void;
  onRemoveTag: (tag: string) => void;
};

function sourceName(exe: string) {
  return exe.replace(/\.exe$/i, "") || "unknown";
}

export function PreviewPane({
  card,
  position,
  allTags,
  copied,
  onCopy,
  onPin,
  onSecret,
  onDelete,
  onAddTag,
  onRemoveTag,
}: Props) {
  if (!card) {
    return (
      <aside
        className="h-full border-l"
        style={{ borderColor: "var(--line)" }}
      />
    );
  }

  const secret = card.kind === "secret";
  const { code } = kindMark(card.kind);
  const app = sourceName(card.source_exe);
  const title = card.window_title.trim();
  const size =
    card.body.length >= 1000
      ? `${(card.body.length / 1000).toFixed(1)}k chars`
      : `${card.body.length} chars`;

  return (
    <aside
      className="relative flex h-full min-h-0 flex-col overflow-hidden border-l"
      style={{ borderColor: "var(--line)" }}
    >
      <div className="pv-grid pointer-events-none absolute inset-0" />
      <span className="ghost-num pointer-events-none absolute -top-6 -right-6">
        {String(position).padStart(2, "0")}
      </span>

      <div className="relative flex items-center gap-2 px-6 pt-4">
        <p
          className="min-w-0 flex-1 truncate font-mono text-[10px]"
          style={{ color: "var(--faint)" }}
        >
          <span style={{ color: "var(--accent)" }}>{code}</span> {app}
          {title && (
            <>
              <span className="opacity-20" aria-hidden="true">
                {" "}
                /{" "}
              </span>
              {title}
            </>
          )}
        </p>
        <div className="flex shrink-0 items-center gap-0.5">
          <button
            type="button"
            className="btn-ico"
            aria-label="Copy"
            title="Copy  ↵"
            onClick={onCopy}
          >
            <IconCopy className="h-4 w-4" />
          </button>
          <button
            type="button"
            className="btn-ico"
            data-on={card.pinned}
            aria-label={card.pinned ? "Unpin" : "Pin"}
            title={card.pinned ? "Unpin  Ctrl P" : "Pin  Ctrl P"}
            onClick={onPin}
          >
            <IconPin className="h-4 w-4" />
          </button>
          {secret ? null : (
            <button
              type="button"
              className="btn-ico"
              aria-label="Mark as secret"
              title="Mark as secret  Ctrl L"
              onClick={onSecret}
            >
              <IconLock className="h-4 w-4" />
            </button>
          )}
          <span
            className="mx-1 h-4 w-px"
            style={{ background: "var(--line)" }}
          />
          <button
            type="button"
            className="btn-ico hover:!text-(--bad)"
            aria-label="Delete"
            title="Delete  Del"
            onClick={onDelete}
          >
            <IconTrash className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="relative mt-6 min-h-0 flex-1 overflow-auto px-6">
        <div
          className="cursor-pointer"
          title="Click to copy"
          onClick={() => {
            // Dragging to select text must not trigger a copy.
            if (window.getSelection()?.toString()) return;
            onCopy();
          }}
        >
          <ClipContent card={card} />
        </div>
        <div className="mt-6 flex flex-wrap items-center gap-4">
          <button type="button" className="btn-go" onClick={onCopy}>
            {copied ? "Copied" : "Copy"}
            <kbd className="keycap">↵</kbd>
          </button>
          <span className="h-5 w-px" style={{ background: "var(--line)" }} />
          <TagEditor
            card={card}
            allTags={allTags}
            onAdd={onAddTag}
            onRemove={onRemoveTag}
          />
        </div>
      </div>

      <div
        className="relative mt-3 grid grid-cols-4 border-t py-3"
        style={{ borderColor: "var(--line)" }}
      >
        {[
          ["copied", card.copy_count === 1 ? "once" : `${card.copy_count}×`],
          ["from", app],
          ["size", secret ? "—" : size],
          ["last", relTime(card.last_seen_at)],
        ].map(([k, v], i) => (
          <div
            key={k}
            className="px-6"
            style={i ? { borderLeft: "1px solid var(--line)" } : undefined}
          >
            <p className="stat-k">{k}</p>
            <p className="stat-v truncate">{v}</p>
          </div>
        ))}
      </div>
    </aside>
  );
}
