/** Trace type codes from design/v2 (Martian Mono). */
export function kindMark(kind: string) {
  switch (kind) {
    case "link":
      return { code: "URL", label: "Link" };
    case "email":
      return { code: "EML", label: "Email" };
    case "phone":
      return { code: "TEL", label: "Phone" };
    case "color":
      return { code: "CLR", label: "Color" };
    case "path":
      return { code: "PTH", label: "Path" };
    case "number":
      return { code: "NUM", label: "Number" };
    case "json":
      return { code: "JSN", label: "JSON" };
    case "code":
      return { code: "SRC", label: "Code" };
    case "secret":
      return { code: "KEY", label: "Secret" };
    default:
      return { code: "TXT", label: "Text" };
  }
}

/** Kinds that are exact strings, so the list shows them in the mono face. */
export function isMonoKind(kind: string) {
  return kind !== "text" && kind !== "secret";
}
