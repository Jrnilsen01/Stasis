import { useEffect, useId, useState } from "react";

import {
  configPath,
  errorText,
  resetSettings,
  revealPath,
  saveSettings,
  type Settings,
} from "../lib/api";

type SettingsViewProps = {
  settings: Settings;
  onChange: (settings: Settings) => void;
};

const INTERVALS = [
  { value: 1000, label: "Every second" },
  { value: 2000, label: "Every 2 seconds" },
  { value: 5000, label: "Every 5 seconds" },
];

/**
 * Only settings that something reads.
 *
 * Both fields here are consumed: the Steam folder is passed to the scanner,
 * and the interval drives the footprint poll. There is deliberately no "launch
 * at startup" or "enable overlay" toggle, because neither has an engine behind
 * it yet, and a switch that flips without doing anything is a dead control
 * dressed up as a feature.
 *
 * The composition is a single narrow column rather than the panels used
 * elsewhere: this screen is read one row at a time, not scanned.
 */
export function SettingsView({ settings, onChange }: SettingsViewProps) {
  const pathFieldId = useId();
  const intervalFieldId = useId();

  const [draft, setDraft] = useState<Settings>(settings);
  const [status, setStatus] = useState<{ text: string; failed: boolean } | null>(null);
  const [filePath, setFilePath] = useState<string | null>(null);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    configPath()
      .then(setFilePath)
      .catch(() => setFilePath(null));
  }, []);

  const dirty =
    draft.steamPathOverride !== settings.steamPathOverride ||
    draft.sampleIntervalMs !== settings.sampleIntervalMs;

  const save = async () => {
    try {
      const saved = await saveSettings(draft);
      onChange(saved);
      setStatus({ text: `Saved to ${filePath ?? "the config file"}.`, failed: false });
    } catch (error) {
      setStatus({ text: `Could not save: ${errorText(error)}`, failed: true });
    }
  };

  const reset = async () => {
    try {
      const defaults = await resetSettings();
      onChange(defaults);
      setDraft(defaults);
      setStatus({ text: "Settings file deleted, defaults restored.", failed: false });
    } catch (error) {
      setStatus({ text: `Could not reset: ${errorText(error)}`, failed: true });
    }
  };

  return (
    <>
      <div className="view-head">
        <h1 className="view-title">Settings</h1>
        <p className="view-note">
          Stored as plain JSON on this machine. Nothing is sent anywhere.
        </p>
      </div>

      <div className="settings">
        <div className="field">
          <label className="field__label" htmlFor={pathFieldId}>
            Steam folder
          </label>
          <p className="field__hint">
            Leave this empty to detect Steam from the registry and the default
            install paths. Set it only when Steam lives somewhere those checks
            do not find.
          </p>
          <input
            id={pathFieldId}
            className="input"
            type="text"
            spellCheck={false}
            placeholder="Detected automatically"
            value={draft.steamPathOverride}
            onChange={(event) =>
              setDraft({ ...draft, steamPathOverride: event.target.value })
            }
          />
        </div>

        <div className="field">
          <label className="field__label" htmlFor={intervalFieldId}>
            Footprint sample rate
          </label>
          <p className="field__hint">
            How often the bar at the bottom re-measures this process. Faster
            sampling costs slightly more of the thing it is measuring.
          </p>
          <select
            id={intervalFieldId}
            className="select"
            value={draft.sampleIntervalMs}
            onChange={(event) =>
              setDraft({ ...draft, sampleIntervalMs: Number(event.target.value) })
            }
          >
            {INTERVALS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        {filePath && (
          <div className="field">
            <span className="field__label">Settings file</span>
            <p className="field__hint">{filePath}</p>
            <button
              type="button"
              className="btn"
              onClick={() =>
                void revealPath(filePath.replace(/[\\/][^\\/]+$/, ""))
              }
            >
              Open the containing folder
            </button>
          </div>
        )}

        {/* Above the buttons, not below them. The actions sit at the bottom of a
            scrolling column, so a message rendered after them lands off-screen
            and the user never sees the confirmation they just earned. */}
        {status && (
          <p
            className={
              status.failed ? "field__status field__status--error" : "field__status"
            }
            role="status"
          >
            {status.text}
          </p>
        )}

        <div className="field__actions">
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => void save()}
            disabled={!dirty}
          >
            {dirty ? "Save changes" : "No changes to save"}
          </button>
          <button type="button" className="btn" onClick={() => void reset()}>
            Restore defaults
          </button>
        </div>
      </div>
    </>
  );
}
