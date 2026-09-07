import { useCallback, useEffect, useId, useRef, useState } from "react";

import {
  checkSteamPath,
  configPath,
  errorText,
  logPath,
  resetSettings,
  revealPath,
  saveSettings,
  type Settings,
  type SteamPathCheck,
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
 * What each answer about the Steam folder means, and what to do about it.
 *
 * `automatic` is missing on purpose: an empty field is the setting working as
 * intended, and the hint above the input already says so. Repeating it as a
 * status would put a message under a field nobody needs to fill in.
 */
const PATH_MESSAGES: Record<Exclude<SteamPathCheck["status"], "automatic">, string> = {
  usable:
    "Found a steamapps folder here. The scan will read this folder and the libraries it lists.",
  missing:
    "Nothing is at that path. Correct it, or clear the field to go back to detecting Steam automatically.",
  notAFolder:
    "That path is a file, not a folder. Point at the folder it sits in, the one with steamapps beside it.",
  noSteamapps:
    "That folder has no steamapps directory inside it, so Steam is not installed there. Point at the folder holding steam.exe, or clear the field to go back to detecting Steam automatically.",
};

type PathCheckState =
  | { phase: "unchecked" }
  | { phase: "checking" }
  | { phase: "checked"; result: SteamPathCheck }
  | { phase: "failed"; message: string };

type FileFieldProps = {
  label: string;
  hint: string;
  path: string;
};

/**
 * A file on disk the user may want to open: named, then reachable.
 *
 * Two of these exist now, so the button says which file it belongs to. Two
 * buttons both called "Open the containing folder" are indistinguishable to
 * anyone reading the page through a screen reader's list of controls.
 */
function FileField({ label, hint, path }: FileFieldProps) {
  return (
    <div className="field">
      <span className="field__label">{label}</span>
      <p className="field__hint">{path}</p>
      <p className="field__hint">{hint}</p>
      <button
        type="button"
        className="btn"
        // Explorer needs a folder. Handed a file it opens the parent anyway on
        // some Windows builds and nothing on others, so the parent is passed.
        onClick={() => void revealPath(path.replace(/[\\/][^\\/]+$/, ""))}
      >
        Open the folder holding the {label.toLowerCase()}
      </button>
    </div>
  );
}

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
  const pathHintId = `${pathFieldId}-hint`;
  const pathCheckId = `${pathFieldId}-check`;

  const [draft, setDraft] = useState<Settings>(settings);
  const [status, setStatus] = useState<{ text: string; failed: boolean } | null>(null);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [logFilePath, setLogFilePath] = useState<string | null>(null);
  const [pathCheck, setPathCheck] = useState<PathCheckState>({ phase: "unchecked" });

  // Blurring a field twice in quick succession leaves two checks in flight, and
  // the disk decides which answers first. Only the newest one is allowed to
  // reach the screen, so the message always describes what is in the field.
  const latestCheck = useRef(0);

  const runPathCheck = useCallback(async (value: string) => {
    const token = ++latestCheck.current;
    setPathCheck({ phase: "checking" });
    try {
      const result = await checkSteamPath(value);
      if (latestCheck.current === token) setPathCheck({ phase: "checked", result });
    } catch (error) {
      if (latestCheck.current === token) {
        setPathCheck({ phase: "failed", message: errorText(error) });
      }
    }
  }, []);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  // Checks the stored value on arrival, so a folder that was fine when it was
  // saved and has since been renamed says so before the user goes looking for
  // their games. Also re-runs after a save or a reset changes the value.
  useEffect(() => {
    void runPathCheck(settings.steamPathOverride);
  }, [settings.steamPathOverride, runPathCheck]);

  useEffect(() => {
    configPath()
      .then(setFilePath)
      .catch(() => setFilePath(null));
    logPath()
      .then(setLogFilePath)
      .catch(() => setLogFilePath(null));
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

  const wrongPath =
    pathCheck.phase === "checked" &&
    pathCheck.result.status !== "automatic" &&
    pathCheck.result.status !== "usable";

  const pathMessage = (() => {
    if (pathCheck.phase === "checking") return "Checking that folder.";
    if (pathCheck.phase === "failed") {
      return `That folder could not be checked: ${pathCheck.message}`;
    }
    if (pathCheck.phase === "checked" && pathCheck.result.status !== "automatic") {
      return PATH_MESSAGES[pathCheck.result.status];
    }
    return "";
  })();

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
          <p className="field__hint" id={pathHintId}>
            Leave this empty to detect Steam from the registry and the default
            install paths. Set it only when Steam lives somewhere those checks
            do not find.
          </p>
          <input
            id={pathFieldId}
            className={wrongPath ? "input input--wrong" : "input"}
            type="text"
            spellCheck={false}
            placeholder="Detected automatically"
            value={draft.steamPathOverride}
            aria-describedby={`${pathHintId} ${pathCheckId}`}
            aria-invalid={wrongPath}
            onChange={(event) => {
              setDraft({ ...draft, steamPathOverride: event.target.value });
              // The old answer describes text that is no longer in the field.
              setPathCheck({ phase: "unchecked" });
            }}
            onBlur={(event) => void runPathCheck(event.target.value)}
          />
          {/* Always in the DOM so the live region is there before it has
              something to announce, and empty until it does. */}
          <p
            id={pathCheckId}
            className={
              wrongPath || pathCheck.phase === "failed"
                ? "field__check field__check--wrong"
                : "field__check"
            }
            role="status"
          >
            {pathMessage}
          </p>
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
          <FileField
            label="Settings file"
            hint="The two preferences above, as JSON. Deleting it restores the defaults."
            path={filePath}
          />
        )}

        {logFilePath && (
          <FileField
            label="Log file"
            hint="What failed and when, written locally and never sent anywhere. Attach it to a bug report. It is capped at a megabyte and starts over when it fills."
            path={logFilePath}
          />
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
          {/* A path that does not resolve is still worth saving: an external
              drive that is unplugged today is plugged in tomorrow. The check
              says what is wrong, it does not take the decision away. */}
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
