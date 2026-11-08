/** Trace type codes from design/v2 (Martian Mono). */
export function kindMark(kind: string) {
  if (kind === "link") return { code: "URL", label: "Link" };
  if (kind === "code") return { code: "SRC", label: "Code" };
  if (kind === "secret") return { code: "KEY", label: "Secret" };
  return { code: "TXT", label: "Text" };
}
