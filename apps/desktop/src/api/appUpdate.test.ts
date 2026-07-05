import { beforeEach, describe, expect, it, vi } from "vitest";

import { checkForAppUpdate, installAvailableAppUpdate } from "./appUpdate";

const isTauriMock = vi.hoisted(() => vi.fn());
const checkMock = vi.hoisted(() => vi.fn());
const relaunchMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: isTauriMock,
}));

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: checkMock,
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: relaunchMock,
}));

describe("app update API", () => {
  beforeEach(() => {
    isTauriMock.mockReset();
    checkMock.mockReset();
    relaunchMock.mockReset();
  });

  it("does not check for updates in browser preview", async () => {
    isTauriMock.mockReturnValue(false);

    await expect(checkForAppUpdate()).resolves.toEqual({
      state: "not_available",
      currentVersion: "browser-preview",
    });

    expect(checkMock).not.toHaveBeenCalled();
  });

  it("returns update metadata when an update is available", async () => {
    isTauriMock.mockReturnValue(true);
    checkMock.mockResolvedValue({
      currentVersion: "0.1.0",
      version: "0.1.1",
      date: "2026-07-05T00:00:00Z",
      body: "Bug fixes",
    });

    await expect(checkForAppUpdate()).resolves.toEqual({
      state: "available",
      currentVersion: "0.1.0",
      version: "0.1.1",
      date: "2026-07-05T00:00:00Z",
      body: "Bug fixes",
    });
  });

  it("normalizes optional update metadata to null", async () => {
    isTauriMock.mockReturnValue(true);
    checkMock.mockResolvedValue({
      currentVersion: "0.1.0",
      version: "0.1.1",
    });

    await expect(checkForAppUpdate()).resolves.toEqual({
      state: "available",
      currentVersion: "0.1.0",
      version: "0.1.1",
      date: null,
      body: null,
    });
  });

  it("downloads, installs, and relaunches an available update", async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    isTauriMock.mockReturnValue(true);
    checkMock.mockResolvedValue({
      currentVersion: "0.1.0",
      version: "0.1.1",
      downloadAndInstall,
    });
    relaunchMock.mockResolvedValue(undefined);

    await expect(installAvailableAppUpdate()).resolves.toEqual({
      state: "installed",
      currentVersion: "0.1.0",
      version: "0.1.1",
    });

    expect(downloadAndInstall).toHaveBeenCalledOnce();
    expect(relaunchMock).toHaveBeenCalledOnce();
  });
});
