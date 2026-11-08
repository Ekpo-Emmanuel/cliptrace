/// <reference types="vite/client" />

import type { ClipCard } from "./api";

export {};

declare global {
  interface Window {
    cliptraceOnOpen?: (cache?: {
      cards?: ClipCard[];
      paused?: boolean;
      keyword_only?: boolean;
    }) => void;
    cliptraceOnHide?: () => void;
  }
}
