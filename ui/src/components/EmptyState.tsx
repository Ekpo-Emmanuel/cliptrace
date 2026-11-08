import { useEffect, useRef, useState } from "react";
import { IconCheck, IconCopy } from "../icons";

type Kind = "history" | "pins" | "results";

type Props = {
  kind: Kind;
  query?: string;
  onExample?: (q: string) => void;
};

const SAMPLE_CLIP = "https://github.com/Ekpo-Emmanuel";
const KBD =
  "font-label-sm kbd-bevel rounded-sm bg-surface-container-high px-1.5 py-0.5 text-on-surface";

function Marker({
  n,
  last,
  active,
}: {
  n: number;
  last?: boolean;
  active?: boolean;
}) {
  return (
    <div className="relative flex w-6 shrink-0 flex-col items-center">
      <span
        className={
          active
            ? "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-on-primary"
            : "flex h-6 w-6 items-center justify-center rounded-full bg-white text-[10px] font-semibold text-on-surface-variant ring-1 ring-outline-variant/45"
        }
      >
        {n}
      </span>
      {last ? null : (
        <span className="mt-1 w-px min-h-[1.25rem] flex-1 bg-outline-variant/35" />
      )}
    </div>
  );
}

export function EmptyState({ kind, query = "", onExample }: Props) {
  const [sampleCopied, setSampleCopied] = useState(false);
  const copiedTimer = useRef(0);

  useEffect(() => {
    return () => window.clearTimeout(copiedTimer.current);
  }, []);

  if (kind === "pins") {
    return (
      <div className="flex h-full flex-col justify-center bg-white px-space-lg">
        <h1 className="font-headline-md text-on-surface">Nothing pinned yet</h1>
        <p className="font-body-sm mt-space-xs max-w-md text-on-surface-variant">
          Pin a clip with <kbd className={KBD}>Ctrl+P</kbd>. Pinned clips skip
          purge.
        </p>
      </div>
    );
  }
  if (kind === "results") {
    const q = query.trim();
    const examples = ["wifi last week", "#work", "from chrome"];
    return (
      <div className="flex h-full flex-col justify-center bg-white px-space-lg">
        <h1 className="font-headline-md text-on-surface">
          No copies for{" "}
          <span className="text-primary">
            “{q.length > 40 ? `${q.slice(0, 40)}…` : q || "this search"}”
          </span>
        </h1>
        <p className="font-body-sm mt-space-xs max-w-md text-on-surface-variant">
          Search stays on this PC. History keeps 90 days. Try a shorter phrase,
          a day, or a tag.
        </p>
        <div className="mt-space-md flex flex-wrap gap-1.5">
          {examples.map((ex) => (
            <button
              key={ex}
              type="button"
              className="font-label-md cursor-pointer rounded-full bg-secondary-container px-space-sm py-1 text-on-secondary-container transition-transform duration-100 ease-out active:scale-[0.97]"
              onClick={() => onExample?.(ex)}
            >
              {ex}
            </button>
          ))}
        </div>
      </div>
    );
  }

  const copySample = async () => {
    try {
      await navigator.clipboard.writeText(SAMPLE_CLIP);
    } catch {
      /* capture watcher still indexes if paste-copy fails in mock */
    }
    setSampleCopied(true);
    window.clearTimeout(copiedTimer.current);
    copiedTimer.current = window.setTimeout(() => setSampleCopied(false), 2000);
  };

  return (
    <div className="grid h-full min-h-0 grid-cols-[minmax(0,40%)_minmax(0,60%)] bg-white">
      <div className="flex min-h-0 flex-col justify-between px-space-lg py-space-lg">
        <div>
          <p className="font-label-xs tracking-[0.16em] text-primary">
            CLIPTRACE
          </p>
          <h1 className="font-headline-xl mt-2 text-on-surface">
            Your clipbord memory
            <br />
            begins now
          </h1>
          <p className="font-body-sm mt-space-sm max-w-[26ch] text-on-surface-variant">
            Cliptrace automatically saves your copied links, code, passwords,
            and notes privately to your local machine. Nothing ever leaves your
            computer.
          </p>
        </div>

        <div>
          <p className="font-label-xs mb-1.5 text-on-surface-variant">
            Try a first clip
          </p>
          <p className="font-body-sm truncate text-on-surface">{SAMPLE_CLIP}</p>
          <button
            type="button"
            onClick={() => void copySample()}
            className="font-label-md mt-space-sm inline-flex w-full cursor-pointer items-center justify-center gap-1.5 rounded bg-primary px-space-sm py-2 text-on-primary transition-transform duration-100 ease-out active:scale-[0.97]"
          >
            {sampleCopied ? (
              <>
                <IconCheck className="h-3.5 w-3.5" />
                Copied
              </>
            ) : (
              <>
                <IconCopy className="h-3.5 w-3.5" />
                Copy a sample
              </>
            )}
          </button>
        </div>
      </div>

      <ol className="flex min-h-0 flex-col justify-center bg-surface-container-low/40 px-space-lg py-space-lg">
        <li className="flex gap-space-sm">
          <Marker n={1} active />
          <div className="min-w-0 pb-space-md">
            <p className="font-headline-sm text-on-surface">Copy normally</p>
            <p className="font-body-sm mt-1 text-on-surface-variant">
              <kbd className={KBD}>Ctrl+C</kbd> in any Windows app. Indexed in
              the background.
            </p>
          </div>
        </li>
        <li className="flex gap-space-sm">
          <Marker n={2} />
          <div className="min-w-0 py-space-xs pb-space-md">
            <p className="font-headline-sm text-on-surface">Summon anytime</p>
            <p className="font-body-sm mt-1 text-on-surface-variant">
              Press the global shortcut to summon this palette atop whatever
              project or document you are in.
            </p>
            <div className="mt-space-xs flex items-center gap-1 mt-2">
              <kbd className={KBD}>Ctrl</kbd>
              <span className="font-label-sm text-outline">+</span>
              <kbd className={KBD}>Alt</kbd>
              <span className="font-label-sm text-outline">+</span>
              <kbd className={KBD}>V</kbd>
            </div>
          </div>
        </li>
        <li className="flex gap-space-sm">
          <Marker n={3} last />
          <div className="min-w-0 pt-space-xs">
            <p className="font-headline-sm text-on-surface">
              Search by meaning
            </p>
            <p className="font-body-sm mt-1 text-on-surface-variant">
              Type a phrase, then <kbd className={KBD}>Enter / Click it</kbd> to
              copy.
            </p>
            <div className="mt-space-xs flex flex-wrap gap-1 mt-2">
              {["Zoom link", "API secret"].map((ex) => (
                <button
                  key={ex}
                  type="button"
                  onClick={() => onExample?.(ex)}
                  className="font-label-sm cursor-pointer rounded-full bg-white px-2 py-0.5 text-on-surface ring-1 ring-outline-variant/40 transition-transform duration-100 ease-out hover:ring-primary/50 active:scale-[0.97]"
                >
                  {ex}
                </button>
              ))}
            </div>
          </div>
        </li>
      </ol>
    </div>
  );
}
