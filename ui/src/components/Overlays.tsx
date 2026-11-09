import type { CliptraceApi } from "../api";
import type { Theme } from "../lib/theme";
import { SecretDialog } from "./SecretDialog";
import { SettingsPanel } from "./SettingsPanel";
import { Welcome } from "./Welcome";

type Props = {
  api: CliptraceApi | null;
  compact: boolean;
  settings: {
    open: boolean;
    theme: Theme;
    onTheme: (theme: Theme) => void;
    onClose: () => void;
    onError: (message: string) => void;
    onShowWelcome: () => void;
  };
  welcome: { open: boolean; hotkey: string; onDone: () => void };
  secret: {
    open: boolean;
    /** The clip text shown in the confirmation, masked by the backend. */
    specimen: string;
    onCancel: () => void;
    onConfirm: () => void;
  };
};

/** Everything that opens on top of the popup: settings, welcome, secret confirm. */
export function Overlays({ api, compact, settings, welcome, secret }: Props) {
  return (
    <>
      {settings.open ? (
        <SettingsPanel
          api={api}
          theme={settings.theme}
          onTheme={settings.onTheme}
          onClose={settings.onClose}
          onError={settings.onError}
          onShowWelcome={settings.onShowWelcome}
        />
      ) : null}
      {welcome.open ? (
        <Welcome
          hotkey={welcome.hotkey}
          compact={compact}
          onDone={welcome.onDone}
        />
      ) : null}
      {secret.open ? (
        <SecretDialog
          specimen={secret.specimen}
          onCancel={secret.onCancel}
          onConfirm={secret.onConfirm}
        />
      ) : null}
    </>
  );
}
