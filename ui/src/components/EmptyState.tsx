import { useState } from "react";
import { IconCheck } from "../icons";

type Kind = "history" | "pins" | "results";

type Props = {
  kind: Kind;
  query?: string;
  total?: number;
  compact?: boolean;
  onClear?: () => void;
  onBrowse?: () => void;
};

const SAMPLE_CLIP = "Cliptrace remembers what you copy, so you don’t have to.";

/** Left column: the trace keeps showing, telling the story of the state. */
function Spine({ kind, landed }: { kind: Kind; landed: boolean }) {
  if (kind === "pins") {
    return (
      <div className="px-1 pt-1">
        <div className="tday">
          <span />
          <span />
          <span>pinned · 0</span>
        </div>
        {[1, 2, 3, 4, 5, 6].map((n) => (
          <div
            key={n}
            className="trow !cursor-default hover:!bg-transparent"
            style={{ opacity: 1 - n * 0.12 }}
          >
            <span
              className="pt-4 pr-2 text-right font-mono text-[10px]"
              style={{ color: "var(--faint)" }}
            >
              {n}
            </span>
            <span className="tspine dash">
              <span className="tnode slot" />
            </span>
            <span />
          </div>
        ))}
      </div>
    );
  }
  if (kind === "results") {
    return (
      <div className="px-1 pt-1">
        <div className="tday">
          <span />
          <span />
          <span>0 matches</span>
        </div>
        <div className="trow !cursor-default hover:!bg-transparent">
          <span />
          <span className="tspine">
            <span className="tlit" style={{ height: 21 }} />
            <span className="tcut" aria-hidden>
              ×
            </span>
          </span>
          <span
            className="pt-[14px] pl-1 font-mono text-[10px]"
            style={{ color: "var(--accent)" }}
          >
            search · no results
          </span>
        </div>
        <div className="trow !cursor-default hover:!bg-transparent">
          <span />
          <span className="tspine dash" />
          <span />
        </div>
      </div>
    );
  }
  return (
    <div className="px-1 pt-1">
      <div className="tday">
        <span />
        <span />
        <span>now</span>
      </div>
      <div className="trow !cursor-default hover:!bg-transparent">
        <span />
        <span className={landed ? "tspine" : "tspine dash"}>
          {landed ? <span className="tlit" style={{ height: 21 }} /> : null}
          <span className={landed ? "tnode sel" : "tnode wait"} />
        </span>
        <div className="pt-[11px] pl-1">
          <p className="text-[14.5px] leading-[1.25] font-medium">
            {landed ? SAMPLE_CLIP : "Waiting for your first copy"}
          </p>
          <p
            className="mt-1 font-mono text-[9.5px]"
            style={{ color: "var(--faint)" }}
          >
            {landed
              ? "TXT · just now"
              : "every copy becomes a point on this line"}
          </p>
        </div>
      </div>
      <div className="trow !cursor-default hover:!bg-transparent">
        <span />
        <span className="tspine dash" />
        <span />
      </div>
    </div>
  );
}

function Row({
  k,
  title,
  hint,
  done = false,
}: {
  k: string;
  title: string;
  hint: string;
  done?: boolean;
}) {
  return (
    <div className="erow" style={done ? { opacity: 0.55 } : undefined}>
      <span
        className="w-6 font-mono text-[10px]"
        style={{ color: "var(--faint)" }}
      >
        {k}
      </span>
      <div className="min-w-0 flex-1">
        <p
          className="text-[14px] font-medium"
          style={done ? { textDecoration: "line-through" } : undefined}
        >
          {title}
        </p>
        <p className="mt-0.5 text-[13px]" style={{ color: "var(--muted)" }}>
          {hint}
        </p>
      </div>
    </div>
  );
}

export function EmptyState({
  kind,
  query = "",
  total = 0,
  compact = false,
  onClear,
  onBrowse,
}: Props) {
  const [sampleCopied, setSampleCopied] = useState(false);

  async function copySample() {
    try {
      await navigator.clipboard.writeText(SAMPLE_CLIP);
    } catch {
      /* the capture watcher still indexes a real copy */
    }
    // Stays set: the real copy lands on the trace and replaces this screen.
    setSampleCopied(true);
  }

  const q = query.trim();
  const shown = q.length > 40 ? `${q.slice(0, 40)}…` : q || "this search";

  if (compact) {
    const head =
      kind === "pins"
        ? "Nothing pinned."
        : kind === "results"
          ? `No clips match “${shown}”`
          : "Nothing on the trace yet.";
    const hint =
      kind === "pins"
        ? "Select a clip, then Ctrl P."
        : kind === "results"
          ? "Try fewer words, a #tag, or a day."
          : "Copy anything. It lands here.";
    return (
      <div className="flex h-full flex-col justify-center px-6">
        <p
          className="mb-2 font-mono text-[10px]"
          style={{ color: "var(--accent)" }}
        >
          {kind === "results" ? "search · no results" : "quick paste"}
        </p>
        <h1 className="m-0 text-[28px] leading-[1.05] font-semibold tracking-[-0.035em] break-words">
          {head}
        </h1>
        <p className="mt-3 text-[13px]" style={{ color: "var(--muted)" }}>
          {hint}
        </p>
        {kind === "results" ? (
          <button
            type="button"
            className="btn-gh mt-4 w-fit !border-(--line)"
            onClick={onClear}
          >
            Clear search
            <kbd className="keycap">esc</kbd>
          </button>
        ) : kind === "pins" ? (
          <button
            type="button"
            className="btn-gh mt-4 w-fit !border-(--line)"
            onClick={onBrowse}
          >
            Browse all clips
          </button>
        ) : null}
      </div>
    );
  }

  return (
    <div className="grid h-full min-h-0 grid-cols-[344px_minmax(0,1fr)]">
      <Spine kind={kind} landed={sampleCopied} />
      <section className="relative flex min-h-0 flex-col justify-center overflow-hidden border-l px-9 py-8"
        style={{ borderColor: "var(--line)" }}
      >
        <div className="pv-grid pointer-events-none absolute inset-0" />
        <div className="relative max-w-[430px]">
          {kind === "history" ? (
            <>
              <p
                className="mb-3 font-mono text-[10px]"
                style={{ color: "var(--accent)" }}
              >
                first run
              </p>
              <h1 className="m-0 text-[42px] leading-[1.02] font-semibold tracking-[-0.035em]">
                {sampleCopied ? (
                  <>
                    There it is.
                    <br />
                    Press Enter to paste it.
                  </>
                ) : (
                  <>
                    Nothing on
                    <br />
                    the trace yet.
                  </>
                )}
              </h1>
              <div className="mt-6">
                <Row
                  k="01"
                  title="Copy anything"
                  hint="Text, links and code, from any app. Ctrl C"
                  done={sampleCopied}
                />
                <Row
                  k="02"
                  title="Open Cliptrace"
                  hint="Press Ctrl Alt V from anywhere."
                />
                <Row
                  k="03"
                  title="Paste it back"
                  hint="Pick a clip and press Enter."
                />
              </div>
              <div className="mt-5 flex items-center gap-4">
                <button
                  type="button"
                  className="btn-gh !border-(--line)"
                  onClick={() => void copySample()}
                >
                  {sampleCopied ? (
                    <>
                      <IconCheck className="h-3.5 w-3.5" />
                      Copied
                    </>
                  ) : (
                    "Copy a sample line"
                  )}
                </button>
                <span
                  className="min-w-0 truncate font-mono text-[10px]"
                  style={{ color: "var(--faint)" }}
                >
                  try it — copy this line
                </span>
              </div>
            </>
          ) : null}

          {kind === "pins" ? (
            <>
              <p
                className="mb-3 font-mono text-[10px]"
                style={{ color: "var(--accent)" }}
              >
                pinned · 0
              </p>
              <h1 className="m-0 text-[42px] leading-[1.02] font-semibold tracking-[-0.035em]">
                Nothing pinned.
              </h1>
              <p
                className="mt-4 text-[15px] leading-[1.5]"
                style={{ color: "var(--muted)" }}
              >
                Pins never fall off the end. Keep the things you paste every
                week: an address, a sign-off, a snippet.
              </p>
              <div className="mt-5">
                <Row
                  k="^P"
                  title="Select any clip, then Ctrl P"
                  hint="Pinned clips skip the purge."
                />
              </div>
              <button
                type="button"
                className="btn-gh mt-5 !border-(--line)"
                onClick={onBrowse}
              >
                Browse all{total ? ` ${total}` : ""} clips
              </button>
            </>
          ) : null}

          {kind === "results" ? (
            <>
              <p
                className="mb-3 font-mono text-[10px]"
                style={{ color: "var(--accent)" }}
              >
                search · no results
              </p>
              <h1 className="m-0 text-[34px] leading-[1.05] font-semibold tracking-[-0.035em] break-words">
                No clips match
                <br />“{shown}”
              </h1>
              <p
                className="mt-4 text-[14px]"
                style={{ color: "var(--muted)" }}
              >
                Narrow it down another way.
              </p>
              <div className="mt-3">
                <Row
                  k="#tag"
                  title="#meeting"
                  hint="Only clips with that tag."
                />
                <Row
                  k="time"
                  title="yesterday · last week"
                  hint="Add a day or a range to the search."
                />
                <Row
                  k="less"
                  title="fewer words"
                  hint="Try one or two words instead."
                />
              </div>
              <p
                className="mt-4 font-mono text-[10px]"
                style={{ color: "var(--faint)" }}
              >
                Secrets are never searched.
              </p>
              <button
                type="button"
                className="btn-gh mt-3 !border-(--line)"
                onClick={onClear}
              >
                Clear search
                <kbd className="keycap">esc</kbd>
              </button>
            </>
          ) : null}
        </div>
      </section>
    </div>
  );
}
