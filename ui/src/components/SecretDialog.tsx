import { IconCheck } from "../icons";

type Props = {
  specimen: string;
  onCancel: () => void;
  onConfirm: () => void;
};

export function SecretDialog({ specimen, onCancel, onConfirm }: Props) {
  const facts = [
    "Encrypted on this device",
    "Masked everywhere and left out of search",
    "Still pastes normally with Enter",
  ];
  return (
    <div className="scrim">
      <div className="dlg" role="dialog" aria-modal="true" aria-labelledby="sec-title">
        <div className="flex flex-col gap-2">
          <span
            className="font-mono text-[9.5px]"
            style={{ color: "var(--accent)" }}
          >
            ctrl l · mark secret
          </span>
          <h2
            id="sec-title"
            className="m-0 text-[25px] leading-[1.1] font-semibold tracking-[-0.03em]"
          >
            Lock this clip as a secret?
          </h2>
        </div>
        <div className="spec">
          <span className="line-clamp-2">{specimen}</span>
          <span className="sweep" aria-hidden="true" />
        </div>
        <div className="flex flex-col gap-2">
          {facts.map((f) => (
            <span key={f} className="fact">
              <IconCheck className="h-3.5 w-3.5" strokeWidth={2} />
              {f}
            </span>
          ))}
        </div>
        <div className="warn">
          <span style={{ color: "var(--bad)" }} aria-hidden>
            ⚠
          </span>
          <span>
            This is one-way. Once locked, the clip can’t be turned back into
            plain text.
          </span>
        </div>
        <div className="flex justify-end gap-2">
          <button type="button" className="btn-gh" onClick={onCancel}>
            Cancel
            <kbd className="keycap">esc</kbd>
          </button>
          <button type="button" className="btn-go" onClick={onConfirm}>
            Encrypt &amp; lock
            <kbd className="keycap">↵</kbd>
          </button>
        </div>
      </div>
    </div>
  );
}
