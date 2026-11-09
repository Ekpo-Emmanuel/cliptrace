import { useEffect, useState, type ReactNode } from "react";
import type { CliptraceApi, Settings, SettingsPatch } from "../api";
import { IconClose } from "../icons";
import type { Theme } from "../lib/theme";

type Props = {
  api: CliptraceApi | null;
  theme: Theme;
  onTheme: (theme: Theme) => void;
  onClose: () => void;
  onError: (message: string) => void;
  onShowWelcome: () => void;
};

function Section({
  label,
  note,
  children,
}: {
  label: string;
  note?: string;
  children: ReactNode;
}) {
  return (
    <section className="mt-6 first:mt-0">
      <div className="flex items-baseline justify-between">
        <h3
          className="m-0 font-mono text-[10px] font-normal"
          style={{ color: "var(--accent)" }}
        >
          {label}
        </h3>
        {note ? (
          <span
            className="font-mono text-[9.5px]"
            style={{ color: "var(--faint)" }}
          >
            {note}
          </span>
        ) : null}
      </div>
      <div className="mt-2">{children}</div>
    </section>
  );
}

function Row({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="erow !items-center !gap-3">
      <div className="min-w-0 flex-1">
        <p className="text-[14px] font-medium">{title}</p>
        {hint ? (
          <p className="mt-0.5 text-[12.5px]" style={{ color: "var(--muted)" }}>
            {hint}
          </p>
        ) : null}
      </div>
      {children}
    </div>
  );
}

function Switch({
  on,
  label,
  onChange,
}: {
  on: boolean;
  label: string;
  onChange: (next: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      className="switch"
      onClick={() => onChange(!on)}
    >
      <span />
    </button>
  );
}

function Choice<T extends string | number>({
  value,
  options,
  label,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  label: string;
  onChange: (next: T) => void;
}) {
  return (
    <div className="seg shrink-0" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={String(o.value)}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          aria-selected={o.value === value}
          className="seg-item"
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

const SHORTCUTS: { title: string; rows: [string[], string][] }[] = [
  {
    title: "move",
    rows: [
      [["↑", "↓"], "Previous / next clip"],
      [["pgup", "pgdn"], "Jump six clips"],
      [["home", "end"], "First / last (empty search)"],
      [["ctrl", "1 / 2"], "All / pinned"],
      [["ctrl", "k"], "Focus search"],
    ],
  },
  {
    title: "act on a clip",
    rows: [
      [["↵"], "Copy and close"],
      [["ctrl", "p"], "Pin / unpin"],
      [["ctrl", "t"], "Add tag"],
      [["ctrl", "l"], "Mark as secret"],
      [["del"], "Delete (undo with ctrl z)"],
    ],
  },
  {
    title: "several at once",
    rows: [
      [["shift", "↑↓"], "Extend selection"],
      [["ctrl", "space"], "Check / uncheck clip"],
      [["ctrl", "a"], "Check all (empty search)"],
      [["ctrl", "p"], "Pin checked clips"],
      [["del"], "Delete checked clips"],
    ],
  },
  {
    title: "window",
    rows: [
      [["tab"], "Quick paste / full view"],
      [["ctrl", ","], "Settings"],
      [["ctrl", "shift", "d"], "Light / dark"],
      [["esc"], "Close, step back"],
    ],
  },
];

const TTL = [
  { value: 1, label: "1h" },
  { value: 6, label: "6h" },
  { value: 24, label: "24h" },
  { value: 72, label: "3d" },
  { value: 4294967295, label: "never" },
];
const KEEP = [
  { value: 30, label: "30d" },
  { value: 90, label: "90d" },
  { value: 180, label: "6mo" },
  { value: 365, label: "1y" },
];

export function SettingsPanel({
  api,
  theme,
  onTheme,
  onClose,
  onError,
  onShowWelcome,
}: Props) {
  const [s, setS] = useState<Settings | null>(null);
  const [app, setApp] = useState("");

  useEffect(() => {
    let live = true;
    void api
      ?.getSettings()
      .then((next) => live && setS(next))
      .catch((err) =>
        onError(err instanceof Error ? err.message : String(err)),
      );
    return () => {
      live = false;
    };
  }, [api]);

  async function save(patch: SettingsPatch) {
    if (!api) return;
    try {
      setS(await api.updateSettings(patch));
    } catch (err) {
      onError(typeof err === "string" ? err : "Could not save the setting");
    }
  }

  function addApp() {
    if (!s) return;
    const name = app.trim().toLowerCase();
    if (!name) return;
    const exe = name.endsWith(".exe") ? name : `${name}.exe`;
    setApp("");
    if (!s.blocklist.includes(exe)) void save({ blocklist: [...s.blocklist, exe] });
  }

  return (
    <div className="settings-wrap" onMouseDown={onClose}>
      <aside
        className="settings"
        role="dialog"
        aria-modal="true"
        aria-labelledby="set-title"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header className="flex items-center gap-3 px-6 pt-5 pb-4">
          <div className="min-w-0 flex-1">
            <h2
              id="set-title"
              className="m-0 text-[25px] leading-[1.1] font-semibold tracking-[-0.03em]"
            >
              Settings
            </h2>
          </div>
          <button
            type="button"
            className="btn-ico"
            aria-label="Close settings"
            onClick={onClose}
          >
            <IconClose className="h-4 w-4" />
          </button>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-6">
          <Section label="appearance">
            <Row title="Theme" hint="System follows Windows.">
              <Choice<Theme>
                label="Theme"
                value={theme}
                options={[
                  { value: "system", label: "system" },
                  { value: "light", label: "light" },
                  { value: "dark", label: "dark" },
                ]}
                onChange={onTheme}
              />
            </Row>
          </Section>

          {s ? (
            <>
              <Section label="capture">
                <Row
                  title="Pause capture"
                  hint="Copies are not saved while paused."
                >
                  <Switch
                    on={s.paused}
                    label="Pause capture"
                    onChange={(paused) => void save({ paused })}
                  />
                </Row>
                <Row
                  title="Launch at sign-in"
                  hint="Start Cliptrace with Windows."
                >
                  <Switch
                    on={s.autostart}
                    label="Launch at sign-in"
                    onChange={(autostart) => void save({ autostart })}
                  />
                </Row>
              </Section>

              <Section label="privacy">
                <Row
                  title="Secrets self-clear"
                  hint="Locked clips are deleted after this long, or never."
                >
                  <Choice<number>
                    label="Secrets self-clear"
                    value={s.secret_ttl_hours}
                    options={TTL}
                    onChange={(secret_ttl_hours) =>
                      void save({ secret_ttl_hours })
                    }
                  />
                </Row>
                <Row
                  title="Keep history for"
                  hint="Older unpinned clips are purged. Pins stay."
                >
                  <Choice<number>
                    label="Keep history for"
                    value={s.retention_days}
                    options={KEEP}
                    onChange={(retention_days) => void save({ retention_days })}
                  />
                </Row>
              </Section>

              <Section label="ignored apps">
                <p
                  className="mb-3 text-[12.5px]"
                  style={{ color: "var(--muted)" }}
                >
                  Copies from these apps are never saved. Password managers
                  that mark their copies are already skipped.
                </p>
                <div className="flex flex-wrap gap-2">
                  {s.blocklist.map((exe) => (
                    <button
                      key={exe}
                      type="button"
                      className="chip"
                      aria-label={`Stop ignoring ${exe}`}
                      onClick={() =>
                        void save({
                          blocklist: s.blocklist.filter((b) => b !== exe),
                        })
                      }
                    >
                      {exe}
                      <span style={{ color: "var(--faint)" }}>×</span>
                    </button>
                  ))}
                  {s.blocklist.length === 0 ? (
                    <span
                      className="font-mono text-[10.5px]"
                      style={{ color: "var(--faint)" }}
                    >
                      none yet
                    </span>
                  ) : null}
                </div>
                <form
                  className="mt-3 flex items-center gap-2"
                  onSubmit={(e) => {
                    e.preventDefault();
                    addApp();
                  }}
                >
                  <label className="sr-only" htmlFor="ignore-app">
                    App to ignore
                  </label>
                  <input
                    id="ignore-app"
                    className="field"
                    placeholder="app.exe"
                    value={app}
                    autoComplete="off"
                    spellCheck={false}
                    maxLength={128}
                    onChange={(e) => setApp(e.target.value)}
                  />
                  <button type="submit" className="btn-gh !h-8 !border-(--line)">
                    Add
                  </button>
                </form>
              </Section>

              <Section label="about">
                <Row title="Open shortcut" hint="Change it in config.toml.">
                  <span className="flex shrink-0 gap-1">
                    {s.hotkey.split("+").map((k) => (
                      <kbd key={k} className="keycap">
                        {k.toLowerCase()}
                      </kbd>
                    ))}
                  </span>
                </Row>
                <Row
                  title="Search"
                  hint={
                    s.keyword_only
                      ? "Keyword only. The embedding runtime is missing."
                      : "Meaning and keywords, on this PC."
                  }
                >
                  <span
                    className="font-mono text-[10.5px]"
                    style={{ color: s.keyword_only ? "var(--bad)" : "var(--faint)" }}
                  >
                    {s.keyword_only ? "keyword-only" : "hybrid"}
                  </span>
                </Row>
                <Row title="Welcome screen" hint="The first-run intro.">
                  <button
                    type="button"
                    className="btn-gh !h-8 !border-(--line)"
                    onClick={onShowWelcome}
                  >
                    Show again
                  </button>
                </Row>
                <div className="erow !block">
                  <p className="text-[14px] font-medium">Data folder</p>
                  <p
                    className="mt-1 font-mono text-[10.5px] break-all select-text"
                    style={{ color: "var(--muted)" }}
                  >
                    {s.data_dir}
                  </p>
                  <p
                    className="mt-2 font-mono text-[9.5px]"
                    style={{ color: "var(--faint)" }}
                  >
                    cliptrace {s.version} · nothing leaves this PC
                  </p>
                </div>
              </Section>
            </>
          ) : (
            <p
              className="mt-6 font-mono text-[10.5px]"
              style={{ color: "var(--faint)" }}
            >
              loading…
            </p>
          )}

          <Section label="keyboard">
            {SHORTCUTS.map((group) => (
              <div key={group.title} className="mb-4 last:mb-0">
                <p
                  className="mb-1 font-mono text-[9.5px]"
                  style={{ color: "var(--faint)" }}
                >
                  {group.title}
                </p>
                {group.rows.map(([keys, what]) => (
                  <div
                    key={what}
                    className="flex items-center justify-between gap-3 py-1.5 text-[13px]"
                  >
                    <span style={{ color: "var(--muted)" }}>{what}</span>
                    <span className="flex shrink-0 gap-1">
                      {keys.map((k) => (
                        <kbd key={k} className="keycap">
                          {k}
                        </kbd>
                      ))}
                    </span>
                  </div>
                ))}
              </div>
            ))}
          </Section>
        </div>
      </aside>
    </div>
  );
}
