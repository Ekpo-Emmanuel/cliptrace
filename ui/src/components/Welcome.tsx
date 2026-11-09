import { IconCheck } from "../icons";

type Props = {
  /** Global shortcut as stored in config, e.g. "Ctrl+Alt+V". */
  hotkey: string;
  compact: boolean;
  onDone: () => void;
};

const STEPS = [
  { k: "01", title: "Copy anything", hint: "Text, links and code, from any app." },
  { k: "02", title: "Open Cliptrace", hint: "Use the shortcut above, from anywhere." },
  { k: "03", title: "Paste it back", hint: "Pick a clip and press Enter." },
];

/** First-run screen. Sits over the popup until the user dismisses it once. */
export function Welcome({ hotkey, compact, onDone }: Props) {
  const keys = hotkey.split("+").filter(Boolean);

  return (
    <div
      className="absolute inset-0 z-50 flex min-h-0 flex-col overflow-hidden"
      style={{ background: "var(--bg)" }}
      role="dialog"
      aria-label="Welcome to Cliptrace"
    >
      <div className="pv-grid pointer-events-none absolute inset-0" />
      <div
        className={`relative flex min-h-0 flex-1 flex-col overflow-y-auto ${
          compact ? "px-6 py-4" : "mx-auto w-full max-w-[500px] px-9 py-8"
        }`}
      >
        {/* my-auto centers the content but, unlike justify-center, never clips its top. */}
        <div className="my-auto">
        <p className="mb-3 font-mono text-[10px]" style={{ color: "var(--accent)" }}>
          welcome
        </p>
        <h1
          className={`m-0 font-semibold tracking-[-0.035em] ${
            compact ? "text-[28px] leading-[1.05]" : "text-[42px] leading-[1.02]"
          }`}
        >
          Cliptrace is running.
        </h1>
        <p
          className={`mt-3 ${compact ? "text-[13px]" : "text-[14.5px]"}`}
          style={{ color: "var(--muted)" }}
        >
          It lives in your tray and keeps everything you copy. Nothing leaves this
          PC.
        </p>

        <div className="mt-5 flex items-center gap-3">
          <span className="flex gap-1.5">
            {keys.map((k) => (
              <kbd key={k} className="keycap !h-7 !min-w-7 !px-2 !text-[11px]">
                {k.toLowerCase()}
              </kbd>
            ))}
          </span>
          <span className="text-[13px]" style={{ color: "var(--muted)" }}>
            opens your clips
          </span>
        </div>

        <div className="mt-5">
          {STEPS.map((step) => (
            <div key={step.k} className={compact ? "erow !py-2.5" : "erow"}>
              <span
                className="w-6 font-mono text-[10px]"
                style={{ color: "var(--faint)" }}
              >
                {step.k}
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-[14px] font-medium">{step.title}</p>
                {compact ? null : (
                  <p className="mt-0.5 text-[13px]" style={{ color: "var(--muted)" }}>
                    {step.hint}
                  </p>
                )}
              </div>
            </div>
          ))}
        </div>

        <div className="mt-5 flex items-center gap-4">
          <button type="button" className="btn-go" onClick={onDone} autoFocus>
            <IconCheck className="h-3.5 w-3.5" />
            Got it
            <kbd className="keycap">↵</kbd>
          </button>
        </div>
        <p
          className="mt-4 font-mono text-[9.5px] leading-[1.5]"
          style={{ color: "var(--faint)" }}
        >
          {compact
            ? "tray icon: open, pause, quit"
            : "tray icon: open, pause, quit · the search model downloads once, then it works offline"}
        </p>
        </div>
      </div>
    </div>
  );
}
