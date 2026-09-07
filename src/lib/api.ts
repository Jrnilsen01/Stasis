import { invoke } from "@tauri-apps/api/core";

/**
 * The product name lives here as a single constant rather than being typed
 * into the markup in six places, so renaming stays a one-line change.
 */
export const PRODUCT_NAME = "Stasis";

export type Footprint = {
  pid: number;
  memoryBytes: number;
  /** Null until a second sample exists: one sample cannot yield a rate. */
  cpuPercent: number | null;
  uptimeMs: number;
};

export type Game = {
  appId: string;
  name: string;
  sizeOnDisk: number | null;
  lastPlayed: number | null;
  library: string;
};

/**
 * Three outcomes, because they are three different screens. Collapsing
 * "Steam is missing" into "no games" would tell the user nothing about which
 * of the two happened.
 */
export type ScanResult =
  | { status: "steamNotFound"; searched: string[]; overridden: boolean }
  | { status: "scanned"; steamRoot: string; libraries: string[]; games: Game[] }
  | { status: "unreadable"; path: string; reason: string };

export type Module = {
  id: string;
  name: string;
  summary: string;
  version: string;
  path: string;
};

export type Settings = {
  steamPathOverride: string;
  sampleIntervalMs: number;
};

export const readFootprint = () => invoke<Footprint>("read_footprint");

export const scanSteamGames = (overridePath: string) =>
  invoke<ScanResult>("scan_steam_games", {
    overridePath: overridePath.trim() === "" ? null : overridePath.trim(),
  });

export const listModules = () => invoke<Module[]>("list_modules");
export const modulesDir = () => invoke<string>("modules_dir");

export const loadSettings = () => invoke<Settings>("load_settings");
export const saveSettings = (settings: Settings) =>
  invoke<Settings>("save_settings", { settings });
export const resetSettings = () => invoke<Settings>("reset_settings");
export const configPath = () => invoke<string>("config_path");

export const revealPath = (path: string) => invoke<void>("reveal_path", { path });

/** Tauri rejects with unknown; every caller wants a string it can display. */
export const errorText = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);
