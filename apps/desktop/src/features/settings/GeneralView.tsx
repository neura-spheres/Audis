import { GroupedList, Row } from "@/components/GroupedList";
import { ErrorNotice } from "@/components/ErrorNotice";
import { SegmentedControl, Switch } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import type { CloseBehavior, StartPage, ThemePreference } from "@/schemas/ipc";

/**
 * General settings. Every control here is wired to the Rust settings store and
 * persists to disk immediately.
 */
export function GeneralView() {
  const { settings, error, update } = useSettings();

  if (error && !settings) return <ErrorNotice error={error} />;

  return (
    <div className="flex flex-col gap-8">
      {error ? <ErrorNotice error={error} /> : null}

      <GroupedList title="Appearance">
        <Row
          label="Theme"
          description="Match Windows, or pick one."
          value={
            <SegmentedControl<ThemePreference>
              label="Theme"
              value={settings?.general.theme ?? "system"}
              options={[
                { id: "light", label: "Light" },
                { id: "dark", label: "Dark" },
                { id: "system", label: "System" },
              ]}
              onChange={(theme) =>
                void update((current) => ({
                  ...current,
                  general: { ...current.general, theme },
                }))
              }
            />
          }
        />
      </GroupedList>

      <GroupedList title="Startup">
        <Row
          label="Open on"
          description="The section Audis shows when it starts."
          value={
            <SegmentedControl<StartPage>
              label="Start page"
              value={settings?.general.startPage ?? "dashboard"}
              options={[
                { id: "dashboard", label: "Dashboard" },
                { id: "sessions", label: "Sessions" },
              ]}
              onChange={(startPage) =>
                void update((current) => ({
                  ...current,
                  general: { ...current.general, startPage },
                }))
              }
            />
          }
        />
      </GroupedList>

      <GroupedList
        title="Window"
        footnote="Keeping Audis in the notification area means closing the window will never end a session by accident."
      >
        <Row
          label="When I close the window"
          value={
            <SegmentedControl<CloseBehavior>
              label="Close behaviour"
              value={settings?.general.closeBehavior ?? "minimizeToTray"}
              options={[
                { id: "minimizeToTray", label: "Keep running" },
                { id: "quit", label: "Quit" },
              ]}
              onChange={(closeBehavior) =>
                void update((current) => ({
                  ...current,
                  general: { ...current.general, closeBehavior },
                }))
              }
            />
          }
        />
        <Row
          label="Show icon in the notification area"
          description="Takes effect the next time Audis starts."
          value={
            <Switch
              label="Show tray icon"
              checked={settings?.general.showTrayIcon ?? true}
              onChange={(showTrayIcon) =>
                void update((current) => ({
                  ...current,
                  general: { ...current.general, showTrayIcon },
                }))
              }
            />
          }
        />
      </GroupedList>
    </div>
  );
}
