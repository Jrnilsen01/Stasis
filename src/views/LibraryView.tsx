import { useCallback, useEffect, useState } from "react";

import { errorText, listModules, modulesDir, revealPath, type Module } from "../lib/api";

type LibraryViewProps = {
  onCountChange: (count: number | null) => void;
};

type LoadState =
  | { phase: "loading" }
  | { phase: "error"; message: string }
  | { phase: "ready"; modules: Module[] };

/**
 * Installed overlay modules.
 *
 * There is no bundled module registry yet, so on a fresh install this list is
 * genuinely empty. It ships that emptiness honestly rather than seeding sample
 * modules: a fake "Crosshair v1.2" that cannot be enabled would look installed
 * without being installed, and every later screen would inherit that lie.
 *
 * The empty state is therefore the screen most people will see, which is why
 * it explains what a module is and offers the one action that changes the
 * situation.
 */
export function LibraryView({ onCountChange }: LibraryViewProps) {
  const [state, setState] = useState<LoadState>({ phase: "loading" });
  const [folder, setFolder] = useState<string | null>(null);

  const load = useCallback(async () => {
    setState({ phase: "loading" });
    try {
      const [modules, dir] = await Promise.all([listModules(), modulesDir()]);
      setFolder(dir);
      setState({ phase: "ready", modules });
      onCountChange(modules.length);
    } catch (error) {
      setState({ phase: "error", message: errorText(error) });
      onCountChange(null);
    }
  }, [onCountChange]);

  useEffect(() => {
    void load();
  }, [load]);

  if (state.phase === "loading") {
    return (
      <div className="state">
        <p className="state__label">READING</p>
        <h2 className="state__title">Checking the modules folder</h2>
        <p className="state__body">
          Looking for module manifests on disk. This reads local files only.
        </p>
      </div>
    );
  }

  if (state.phase === "error") {
    return (
      <div className="state">
        <p className="state__label state__label--error">FAILED</p>
        <h2 className="state__title">The modules folder could not be read</h2>
        <p className="state__body">{state.message}</p>
        <div className="state__actions">
          <button type="button" className="btn btn--primary" onClick={() => void load()}>
            Try reading again
          </button>
          {folder && (
            <button type="button" className="btn" onClick={() => void revealPath(folder)}>
              Open the folder
            </button>
          )}
        </div>
      </div>
    );
  }

  if (state.modules.length === 0) {
    return (
      <div className="state">
        <p className="state__label">NOTHING INSTALLED</p>
        <h2 className="state__title">No modules yet</h2>
        <p className="state__body">
          A module is one overlay panel: a crosshair, a timer, a capture
          trigger. Each lives in its own folder with a{" "}
          <code>module.json</code> manifest, and this screen lists whatever is
          on disk. Nothing ships preinstalled, so the folder is empty until you
          put something in it.
        </p>
        <div className="state__actions">
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => void revealPath(folder ?? "")}
            disabled={!folder}
          >
            Open the modules folder
          </button>
          <button type="button" className="btn" onClick={() => void load()}>
            Check again
          </button>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="view-head">
        <h1 className="view-title">Modules</h1>
        <p className="view-note">
          Read from {folder}. Each entry is a folder containing a{" "}
          <code>module.json</code> manifest.
        </p>
      </div>
      <ul className="rows">
        {state.modules.map((module) => (
          <li key={module.id} className="row">
            <span className="row__name">{module.name}</span>
            <span className="row__meta">{module.summary}</span>
            <span className="row__figure">{module.version}</span>
          </li>
        ))}
      </ul>
    </>
  );
}
