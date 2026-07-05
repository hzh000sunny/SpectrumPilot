import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

export type AppUpdateStatus =
  | {
      state: "not_available";
      currentVersion: string;
    }
  | {
      state: "available";
      currentVersion: string;
      version: string;
      date: string | null;
      body: string | null;
    }
  | {
      state: "installed";
      currentVersion: string;
      version: string;
    };

export async function checkForAppUpdate(): Promise<AppUpdateStatus> {
  if (!isTauri()) {
    return {
      state: "not_available",
      currentVersion: "browser-preview",
    };
  }

  const update = await check();
  if (!update) {
    return {
      state: "not_available",
      currentVersion: "0.1.0",
    };
  }

  return {
    state: "available",
    currentVersion: update.currentVersion,
    version: update.version,
    date: update.date ?? null,
    body: update.body ?? null,
  };
}

export async function installAvailableAppUpdate(): Promise<AppUpdateStatus> {
  if (!isTauri()) {
    return {
      state: "not_available",
      currentVersion: "browser-preview",
    };
  }

  const update = await check();
  if (!update) {
    return {
      state: "not_available",
      currentVersion: "0.1.0",
    };
  }

  await update.downloadAndInstall();
  await relaunch();
  return {
    state: "installed",
    currentVersion: update.currentVersion,
    version: update.version,
  };
}
