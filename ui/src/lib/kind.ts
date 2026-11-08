import {
  IconCode,
  IconFileText,
  IconLink,
  IconLock,
} from "../icons";

export function kindMark(kind: string) {
  if (kind === "link") {
    return {
      Icon: IconLink,
      label: "Link",
      chip: "bg-primary/15 text-primary",
    };
  }
  if (kind === "code") {
    return {
      Icon: IconCode,
      label: "Code",
      chip: "bg-secondary-container text-on-secondary-container",
    };
  }
  if (kind === "secret") {
    return {
      Icon: IconLock,
      label: "Secret",
      chip: "bg-tertiary-container text-on-tertiary-container",
    };
  }
  return {
    Icon: IconFileText,
    label: "Text",
    chip: "bg-surface-container-high text-on-surface-variant",
  };
}
