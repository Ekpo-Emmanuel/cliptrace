import { useEffect, useState } from "react";
import { getApi, type CliptraceApi } from "../api";

/** The backend client (Tauri or the browser mock). Null until it has loaded. */
export function useApi(): CliptraceApi | null {
  const [api, setApi] = useState<CliptraceApi | null>(null);
  useEffect(() => {
    let live = true;
    void getApi().then((client) => {
      if (live) setApi(client);
    });
    return () => {
      live = false;
    };
  }, []);
  return api;
}
