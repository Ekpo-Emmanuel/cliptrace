export function relTime(ms: number): string {
  const delta = Date.now() - ms;
  const min = Math.round(delta / 60_000);
  if (min < 1) return "just now";
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  return `${day}d ago`;
}

export function listDate(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = (start(now) - start(d)) / 86_400_000;
  if (diff === 0) return relTime(ms);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function dayLabel(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = (start(now) - start(d)) / 86_400_000;
  if (diff === 0) return "Today";
  if (diff === 1) return "Yesterday";
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

/** 24h clock, e.g. "04:28". */
export function clock(ms: number): string {
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Lower-case mono day heading, e.g. "today · sun 08 nov". */
export function dayHeading(ms: number): string {
  const d = new Date(ms);
  const wk = d.toLocaleDateString("en-GB", { weekday: "short" }).toLowerCase();
  const mon = d.toLocaleDateString("en-GB", { month: "short" }).toLowerCase();
  const date = `${wk} ${String(d.getDate()).padStart(2, "0")} ${mon}`;
  const label = dayLabel(ms);
  if (label === "Today" || label === "Yesterday") {
    return `${label.toLowerCase()} · ${date}`;
  }
  return date;
}
