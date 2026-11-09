import type { ClipCard } from "../api";

type Body = { body: string };

const muted = { color: "var(--muted)" } as const;

/** The big headline used by the structured types (link host, email name...). */
const HEADLINE =
  "leading-none font-extrabold tracking-[-0.04em] [font-variation-settings:'opsz'_96]";

function parseLink(raw: string) {
  try {
    return new URL(raw.trim());
  } catch {
    return null;
  }
}

function Secret({ card }: { card: ClipCard }) {
  return (
    <div>
      <span className="hatch" style={{ width: 220 }} />
      <span className="hatch mt-2" style={{ width: 150 }} />
      <p className="mt-4 font-mono text-[10.5px]" style={muted}>
        {card.preview}
      </p>
      <p className="mt-1 font-mono text-[10px]" style={{ color: "var(--faint)" }}>
        encrypted · kept out of search
      </p>
    </div>
  );
}

function CodeLines({ text }: { text: string }) {
  return (
    <table className="border-collapse font-mono text-[11.5px] leading-[1.95]">
      <tbody>
        {text.split("\n").map((line, i) => (
          <tr key={i}>
            <td
              className="w-8 pr-3 text-right select-none"
              style={{ color: "var(--accent)" }}
            >
              {i + 1}
            </td>
            <td className="whitespace-pre" style={{ color: "var(--text)" }}>
              {line || " "}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Pretty-printed for reading only; Copy still pastes the original text. */
function Json({ body }: Body) {
  let shown = body;
  try {
    shown = JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    /* Not valid JSON after all: show it as plain code. */
  }
  return <CodeLines text={shown} />;
}

function Link({ body }: Body) {
  const url = parseLink(body);
  return (
    <div>
      <p className={`text-[46px] ${HEADLINE}`}>
        {url ? url.host.replace(/^www\./, "") : body}
      </p>
      <p className="mt-4 font-mono text-[11.5px] leading-[1.6] break-all" style={muted}>
        {body}
      </p>
    </div>
  );
}

function Email({ body }: Body) {
  const text = body.trim();
  const at = text.lastIndexOf("@");
  return (
    <div>
      <p className={`text-[46px] break-all ${HEADLINE}`}>{text.slice(0, at)}</p>
      <p className="mt-4 font-mono text-[15px] break-all" style={muted}>
        {text.slice(at)}
      </p>
    </div>
  );
}

function Phone({ body }: Body) {
  return (
    <p className="font-mono text-[34px] leading-[1.15] font-medium tracking-[-0.02em] break-all tabular-nums">
      {body.trim()}
    </p>
  );
}

function NumberFigure({ body }: Body) {
  return (
    <p className="font-mono text-[52px] leading-none font-medium tracking-[-0.03em] break-all tabular-nums">
      {body.trim()}
    </p>
  );
}

/** `#rrggbb` as rgb(), for the line under the swatch. Other formats show as typed. */
function hexToRgb(value: string) {
  const hex = value.slice(1);
  const full =
    hex.length <= 4
      ? hex
          .split("")
          .map((c) => c + c)
          .join("")
      : hex;
  const n = (i: number) => parseInt(full.slice(i, i + 2), 16);
  return full.length === 8
    ? `rgba(${n(0)}, ${n(2)}, ${n(4)}, ${(n(6) / 255).toFixed(2)})`
    : `rgb(${n(0)}, ${n(2)}, ${n(4)})`;
}

function Color({ body }: Body) {
  const value = body.trim();
  const valid = typeof CSS !== "undefined" && CSS.supports("color", value);
  return (
    <div>
      <div
        className="h-[84px] w-full max-w-[320px] rounded-(--r-md) border"
        style={{ background: valid ? value : "transparent", borderColor: "var(--line)" }}
        aria-label={`Color ${value}`}
      />
      <p className="mt-5 font-mono text-[34px] leading-none font-medium tracking-[-0.02em] break-all">
        {value}
      </p>
      {value.startsWith("#") ? (
        <p className="mt-2 font-mono text-[12px]" style={muted}>
          {hexToRgb(value)}
        </p>
      ) : null}
    </div>
  );
}

/** The file or folder name large, with the folder it sits in underneath. */
function Path({ body }: Body) {
  const text = body.trim();
  const parts = text.split(/[\\/]+/).filter(Boolean);
  const name = parts[parts.length - 1] ?? text;
  const dir = text.slice(0, Math.max(0, text.length - name.length)).replace(/[\\/]+$/, "");
  return (
    <div>
      <p className={`text-[40px] break-all ${HEADLINE}`}>{name}</p>
      {dir ? (
        <p className="mt-4 font-mono text-[11.5px] leading-[1.6] break-all" style={muted}>
          {dir}
        </p>
      ) : null}
    </div>
  );
}

function Prose({ body }: Body) {
  const long = body.length > 160;
  return (
    <p
      className={`whitespace-pre-wrap break-words ${
        long
          ? "text-[18px] leading-[1.4] font-medium tracking-[-0.01em]"
          : "text-[31px] leading-[1.16] font-medium tracking-[-0.022em] [font-variation-settings:'opsz'_72]"
      }`}
    >
      {body}
    </p>
  );
}

/** The large preview of a clip. Each kind has its own look. */
export function ClipContent({ card }: { card: ClipCard }) {
  switch (card.kind) {
    case "secret":
      return <Secret card={card} />;
    case "code":
      return <CodeLines text={card.body} />;
    case "json":
      return <Json body={card.body} />;
    case "link":
      return <Link body={card.body} />;
    case "email":
      return <Email body={card.body} />;
    case "phone":
      return <Phone body={card.body} />;
    case "number":
      return <NumberFigure body={card.body} />;
    case "color":
      return <Color body={card.body} />;
    case "path":
      return <Path body={card.body} />;
    default:
      return <Prose body={card.body} />;
  }
}
