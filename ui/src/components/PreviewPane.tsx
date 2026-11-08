import { useEffect, useState, type MouseEvent } from "react";
import type { ClipCard, TagCount } from "../api";
import {
  IconCopy,
  IconEye,
  IconEyeOff,
  IconLink,
  IconLock,
  IconPin,
  IconShield,
  IconTrash,
} from "../icons";
import { kindMark } from "../lib/kind";
import { TagEditor } from "./TagEditor";

type Props = {
  card: ClipCard | undefined;
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
  return exe.replace(/\.exe$/i, "") || "Unknown";
}

function copiedLabel(ms: number, count: number) {
  const d = new Date(ms);
  const date = d.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  const time = d.toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });
  const times = count === 1 ? "once" : `${count} times`;
  return `Copied ${date} · ${time} · ${times}`;
}

function CopyBtn({
  onCopy,
  copied,
  inverse,
}: {
  onCopy: () => void;
  copied: boolean;
  inverse?: boolean;
}) {
  return (
    <button
      type="button"
      className={`font-label-xs inline-flex shrink-0 cursor-pointer items-center gap-1 rounded-md px-2 py-1 ${
        inverse
          ? "text-primary-fixed-dim hover:bg-white/10"
          : "text-primary hover:bg-primary/10"
      }`}
      onClick={(e) => {
        e.stopPropagation();
        onCopy();
      }}
    >
      <IconCopy className="h-3 w-3" />
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

function bodyClick(onCopy: () => void) {
  return (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest("button")) return;
    onCopy();
  };
}

function parseLink(raw: string) {
  try {
    return new URL(raw.trim());
  } catch {
    return null;
  }
}

function SecretBody({
  card,
  copied,
  onCopy,
}: {
  card: ClipCard;
  copied: boolean;
  onCopy: () => void;
}) {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    setShown(false);
  }, [card.id]);

  return (
    <div
      className="cursor-pointer rounded-md bg-amber/10 p-space-sm"
      onClick={bodyClick(onCopy)}
    >
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className="font-label-xs uppercase tracking-wide text-tertiary">
          Secret
        </span>
        <span className="font-label-xs rounded-full bg-tertiary-container px-1.5 py-px text-on-tertiary-container">
          Sensitive
        </span>
      </div>
      <div className="flex items-center justify-between gap-2">
        <p
          className={
            shown
              ? "font-mono min-w-0 flex-1 break-all text-[12px] text-tertiary"
              : "font-mono min-w-0 flex-1 text-[15px] tracking-[0.2em] text-tertiary"
          }
        >
          {shown ? card.preview : "••••••••••••••••"}
        </p>
        <button
          type="button"
          className="rounded-md p-1 text-tertiary hover:bg-amber/20"
          aria-label={shown ? "Hide secret hint" : "Show secret hint"}
          onClick={() => setShown((v) => !v)}
        >
          {shown ? (
            <IconEyeOff className="h-3.5 w-3.5" />
          ) : (
            <IconEye className="h-3.5 w-3.5" />
          )}
        </button>
        <CopyBtn onCopy={onCopy} copied={copied} />
      </div>
    </div>
  );
}

function BodyView({
  card,
  copied,
  onCopy,
}: {
  card: ClipCard;
  copied: boolean;
  onCopy: () => void;
}) {
  if (card.kind === "secret") {
    return <SecretBody card={card} copied={copied} onCopy={onCopy} />;
  }

  if (card.kind === "code") {
    const lines = card.body.split("\n");
    return (
      <div
        className="cursor-pointer overflow-hidden rounded-md bg-slate text-inverse-on-surface"
        onClick={bodyClick(onCopy)}
      >
        <div className="flex items-center justify-between border-b border-white/10 px-3 py-1.5">
          <span className="font-label-xs uppercase tracking-wide text-inverse-on-surface/60">
            Code
          </span>
          <CopyBtn onCopy={onCopy} copied={copied} inverse />
        </div>
        <div className="max-h-56 overflow-auto">
          <table className="w-full border-collapse font-mono text-[12px] leading-5">
            <tbody>
              {lines.map((line, i) => (
                <tr key={i} className="hover:bg-white/5">
                  <td className="w-8 select-none px-2 py-0 text-right text-inverse-on-surface/35">
                    {i + 1}
                  </td>
                  <td className="whitespace-pre px-2 py-0">{line || " "}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  }

  if (card.kind === "link") {
    const url = parseLink(card.body);
    return (
      <div
        className="cursor-pointer rounded-md border border-primary/20 bg-primary/5 p-space-sm"
        onClick={bodyClick(onCopy)}
      >
        <div className="mb-1.5 flex items-center justify-between gap-2">
          <span className="font-label-xs uppercase tracking-wide text-primary">
            Link
          </span>
          <CopyBtn onCopy={onCopy} copied={copied} />
        </div>
        <div className="flex items-start gap-2">
          <IconLink className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
          <div className="min-w-0">
            <p className="font-label-lg truncate text-on-surface">
              {url ? url.host : card.body}
            </p>
            {url ? (
              <p className="font-body-sm break-all text-on-surface-variant">
                {url.pathname}
                {url.search}
              </p>
            ) : null}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className="cursor-pointer rounded-md bg-surface-container-low/80 p-space-sm"
      onClick={bodyClick(onCopy)}
    >
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className="font-label-xs uppercase tracking-wide text-on-surface-variant">
          Text
        </span>
        <CopyBtn onCopy={onCopy} copied={copied} />
      </div>
      <p className="font-body-md whitespace-pre-wrap break-words text-on-surface">
        {card.body}
      </p>
    </div>
  );
}

export function PreviewPane({
  card,
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
    return <aside className="h-full bg-surface-container-low/50" />;
  }

  const secret = card.kind === "secret";
  const { Icon, label, chip } = kindMark(card.kind);
  const title = card.window_title.trim() || label;
  const app = sourceName(card.source_exe);
  return (
    <aside className="flex h-full min-h-0 flex-col border-l border-outline-variant/25 bg-surface-container-lowest">
      <div className="flex items-start gap-2.5 px-space-md pt-space-md">
        <span
          className={`mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full ${chip}`}
        >
          <Icon className="h-4 w-4" />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="font-label-lg truncate text-on-surface">{title}</h2>
          <p className="font-label-xs text-on-surface-variant">{app}</p>
        </div>
        <button
          type="button"
          className={`rounded-md p-1.5 ${
            card.pinned
              ? "bg-primary/10 text-primary"
              : "text-on-surface-variant hover:bg-surface-container"
          }`}
          aria-label={card.pinned ? "Unpin" : "Pin"}
          onClick={onPin}
        >
          <IconPin className="h-4 w-4" />
        </button>
      </div>

      <div className="font-label-xs mt-2 flex flex-wrap items-center gap-2 px-space-md text-on-surface-variant">
        <span>{copiedLabel(card.last_seen_at, card.copy_count)}</span>
        {secret ? (
          <span className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-2 py-0.5 text-primary">
            <IconShield className="h-3 w-3" />
            Encrypted
          </span>
        ) : null}
      </div>

      <div className="mt-space-md min-h-0 flex-1 overflow-auto px-space-md">
        <BodyView card={card} copied={copied} onCopy={onCopy} />
        <div className="mt-space-sm">
          <TagEditor
            card={card}
            allTags={allTags}
            onAdd={onAddTag}
            onRemove={onRemoveTag}
          />
        </div>
      </div>

      <div className="mt-auto flex justify-end px-space-md py-space-sm">
        <div className="font-label-lg inline-flex items-center gap-px rounded-full bg-surface-container-low p-0.5">
          <button
            type="button"
            className={`inline-flex h-8 cursor-pointer items-center gap-1 rounded-full px-3 ${
              card.pinned
                ? "text-tertiary"
                : "text-on-surface-variant hover:bg-surface-container-highest"
            }`}
            onClick={onPin}
          >
            <IconPin className="h-4 w-4" />
            {card.pinned ? "Unpin" : "Pin"}
          </button>
          {secret ? null : (
            <button
              type="button"
              className="inline-flex h-8 cursor-pointer items-center gap-1 rounded-full px-3 text-on-surface-variant hover:bg-surface-container-highest"
              onClick={onSecret}
            >
              <IconLock className="h-4 w-4" />
              Hide
            </button>
          )}
          <button
            type="button"
            className="inline-flex h-8 cursor-pointer items-center gap-1 rounded-full px-3 text-error hover:bg-error-container"
            onClick={onDelete}
          >
            <IconTrash className="h-3.5 w-3.5" />
            Delete
          </button>
        </div>
      </div>
    </aside>
  );
}
