import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsPage } from "./SettingsPage";

const invokeMock = vi.hoisted(() => vi.fn());
const isTauriMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

function runtimePathsMock(overrides: Partial<Record<string, string>> = {}) {
  return {
    appStorageDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot",
    configDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\config",
    metadataDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\metadata",
    internalCacheDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\cache",
    logsDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\logs",
    workspaceRoot: "D:\\SpectrumPilotWorkspace",
    threeGppWorkspaceDir: "D:\\SpectrumPilotWorkspace\\3gpp",
    threeGppInternalCacheDir:
      "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\cache\\3gpp",
    threeGppCatalogDir:
      "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\metadata\\3gpp\\catalog",
    appDataDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot",
    appCacheDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\cache",
    appLogDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\logs",
    threeGppCacheDir: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\cache\\3gpp",
    ...overrides,
  };
}

function catalogStatusMock(overrides: Record<string, unknown> = {}) {
  return {
    catalogRoot: "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\metadata\\3gpp\\catalog",
    manifestCount: 7,
    recordCount: 2696,
    indexCount: 2,
    catalogInstallState: "ready",
    catalogDownloadSource:
      "https://github.com/hzh000sunny/SpectrumPilot/releases/download/catalog/3gpp-compact.json",
    catalogDownloadVersion: "compact-stage-seed-2026-07-05",
    catalogDownloadLastAttemptAt: "2026-07-05T08:00:00Z",
    catalogDownloadLastSuccessAt: "2026-07-05T08:01:00Z",
    catalogDownloadLastError: null,
    catalogDownloadedBytes: 18 * 1024 * 1024,
    catalogDownloadExpectedBytes: 18 * 1024 * 1024,
    catalogDownloadSha256: "abc123",
    seedVersion: "stage-seed-2026-07-02",
    seedGeneratedAt: "2026-07-02T00:00:00Z",
    seedScope: "RAN2 meetings TSGR2_132 and TSGR2_133bis",
    backgroundRefreshEnabled: true,
    backgroundRefreshIntervalMinutes: 60,
    backgroundRefreshTrackedRoots: 6,
    backgroundRefreshMeetingWindow: 8,
    backgroundRefreshState: "succeeded",
    backgroundRefreshLastStartedAt: "2026-07-03T08:00:00Z",
    backgroundRefreshLastCompletedAt: "2026-07-03T08:01:00Z",
    backgroundRefreshLastError: null,
    backgroundRefreshLastRefreshedManifestCount: 12,
    backgroundRefreshLogPath:
      "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\logs\\3gpp-refresh.log",
    lastCheckedAt: "2026-07-01T08:00:00Z",
    ...overrides,
  };
}

describe("SettingsPage", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReturnValue(true);
  });

  it("shows scheduled update, catalog health, and storage paths", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(
          catalogStatusMock({
            backgroundRefreshState: "failed",
            backgroundRefreshLastCompletedAt: null,
            backgroundRefreshLastError: "HTTP 429",
            backgroundRefreshLastRefreshedManifestCount: 0,
          }),
        );
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    const settingsNav = await screen.findByRole("navigation", { name: "Settings sections" });
    expect(within(settingsNav).queryByText("General")).not.toBeInTheDocument();
    expect(within(settingsNav).queryByText("Modules")).not.toBeInTheDocument();
    expect(within(settingsNav).getByRole("button", { name: "System" })).toBeInTheDocument();
    expect(within(settingsNav).getByRole("button", { name: "3GPP Ftp" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(within(settingsNav).queryByRole("button", { name: "Workspace" })).not.toBeInTheDocument();
    expect(
      within(settingsNav).queryByRole("button", { name: "Diagnostics" }),
    ).not.toBeInTheDocument();
    expect(settingsNav.querySelector(".anticon")).not.toBeInTheDocument();

    expect(await screen.findByRole("heading", { name: "Scheduled Update" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Catalog" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Data Locations" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Settings" })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "3GPP Storage" })).not.toBeInTheDocument();
    expect(await screen.findByRole("switch", { name: "Enable scheduled update" })).toBeInTheDocument();
    expect(await screen.findByText("Failed")).toBeInTheDocument();
    expect(await screen.findByText("HTTP 429")).toBeInTheDocument();
    expect(await screen.findByText("stage-seed-2026-07-02")).toBeInTheDocument();
    expect(await screen.findByText("compact-stage-seed-2026-07-05")).toBeInTheDocument();
    expect(await screen.findAllByText("Ready")).toHaveLength(2);
    expect(await screen.findByText("18.0 MB / 18.0 MB")).toBeInTheDocument();
    expect(await screen.findByText("RAN2 meetings TSGR2_132 and TSGR2_133bis")).toBeInTheDocument();
    expect(await screen.findByText("Every 60 minutes")).toBeInTheDocument();
    expect(await screen.findByText("6 tracked roots")).toBeInTheDocument();
    expect(await screen.findByText("8 recent meetings per changed workgroup")).toBeInTheDocument();
    expect(await screen.findByText("Last refresh started")).toBeInTheDocument();
    expect(screen.queryByText("2026-07-03T08:00:00Z")).not.toBeInTheDocument();
    expect(await screen.findByText("Last refresh completed")).toBeInTheDocument();
    expect(await screen.findByText("Not completed")).toBeInTheDocument();
    expect(await screen.findByText("Refresh log")).toBeInTheDocument();
    expect(
      await screen.findByText(
        "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\logs\\3gpp-refresh.log",
      ),
    ).toBeInTheDocument();
    expect(await screen.findByText("7 manifests")).toBeInTheDocument();
    expect(await screen.findByText("2696 indexed TDocs")).toBeInTheDocument();
    expect(await screen.findByText("2 index shards")).toBeInTheDocument();
    expect(
      await screen.findByText(
        "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\metadata\\3gpp\\catalog",
      ),
    ).toBeInTheDocument();
    expect(await screen.findByText("D:\\SpectrumPilotWorkspace\\3gpp")).toBeInTheDocument();
  });

  it("keeps workspace paths inside System settings", async () => {
    const user = userEvent.setup();

    invokeMock.mockImplementation((command: string) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(catalogStatusMock());
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    const settingsNav = await screen.findByRole("navigation", { name: "Settings sections" });
    await user.click(within(settingsNav).getByRole("button", { name: "System" }));

    expect(await screen.findByRole("heading", { name: "System" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Workspace" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Application Storage" })).toBeInTheDocument();
    expect(await screen.findByText("Workspace root")).toBeInTheDocument();
    expect(await screen.findByText("D:\\SpectrumPilotWorkspace")).toBeInTheDocument();
    expect(await screen.findByText("Internal cache")).toBeInTheDocument();
    expect(await screen.findByText("Metadata")).toBeInTheDocument();
  });

  it("saves a changed workspace root from System settings", async () => {
    const user = userEvent.setup();
    const nextPaths = runtimePathsMock({
      workspaceRoot: "E:\\WirelessResearch",
      threeGppWorkspaceDir: "E:\\WirelessResearch\\3gpp",
    });

    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(catalogStatusMock());
      }
      if (command === "set_workspace_root") {
        expect(args).toEqual({ workspaceRoot: "E:\\WirelessResearch" });
        return Promise.resolve(nextPaths);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    const settingsNav = await screen.findByRole("navigation", { name: "Settings sections" });
    await user.click(within(settingsNav).getByRole("button", { name: "System" }));
    const workspaceInput = await screen.findByRole("textbox", { name: "Workspace root" });
    await user.clear(workspaceInput);
    await user.type(workspaceInput, "E:\\WirelessResearch");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_workspace_root", {
        workspaceRoot: "E:\\WirelessResearch",
      });
    });
    expect(await screen.findByText("E:\\WirelessResearch")).toBeInTheDocument();
    expect(await screen.findByText("E:\\WirelessResearch\\3gpp")).toBeInTheDocument();
  });

  it("toggles scheduled update from Settings", async () => {
    const user = userEvent.setup();
    const enabledStatus = catalogStatusMock();
    const disabledStatus = {
      ...enabledStatus,
      backgroundRefreshEnabled: false,
      backgroundRefreshState: "disabled",
    };
    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(enabledStatus);
      }
      if (command === "set_gpp_background_refresh_enabled") {
        expect(args).toEqual({ enabled: false });
        return Promise.resolve(disabledStatus);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    const toggle = await screen.findByRole("switch", { name: "Enable scheduled update" });
    expect(toggle).toBeChecked();

    await user.click(toggle);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_gpp_background_refresh_enabled", {
        enabled: false,
      });
    });
    expect(await screen.findAllByText("Disabled")).toHaveLength(2);
  });

  it("updates scheduled refresh interval and shows refresh log tail", async () => {
    const user = userEvent.setup();
    const updatedStatus = catalogStatusMock({
      backgroundRefreshIntervalMinutes: 30,
      backgroundRefreshState: "succeeded",
      backgroundRefreshLastRefreshedManifestCount: 4,
    });
    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(catalogStatusMock());
      }
      if (command === "set_gpp_background_refresh_interval_minutes") {
        expect(args).toEqual({ intervalMinutes: 30 });
        return Promise.resolve(updatedStatus);
      }
      if (command === "gpp_refresh_log_tail") {
        expect(args).toEqual({ lineCount: 80 });
        return Promise.resolve([
          "2026-07-04T08:00:00Z started manual refresh; interval_minutes=30",
          "2026-07-04T08:01:00Z succeeded manual refresh; refreshed_manifest_count=4",
        ]);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    const intervalInput = await screen.findByRole("spinbutton", { name: "Refresh interval minutes" });
    await user.clear(intervalInput);
    await user.type(intervalInput, "30");
    await user.click(screen.getByRole("button", { name: "Save interval" }));
    await user.click(screen.getByRole("button", { name: "Show log" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_gpp_background_refresh_interval_minutes", {
        intervalMinutes: 30,
      });
    });
    expect(await screen.findByText("Every 30 minutes")).toBeInTheDocument();
    expect(await screen.findByText(/started manual refresh/)).toBeInTheDocument();
    expect(await screen.findByText(/succeeded manual refresh/)).toBeInTheDocument();
  });

  it("runs a manual refresh from Settings", async () => {
    const user = userEvent.setup();
    const refreshedStatus = catalogStatusMock({
      backgroundRefreshState: "succeeded",
      backgroundRefreshLastRefreshedManifestCount: 5,
    });
    invokeMock.mockImplementation((command: string) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(catalogStatusMock());
      }
      if (command === "run_gpp_background_refresh_once") {
        return Promise.resolve(refreshedStatus);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    await user.click(await screen.findByRole("button", { name: "Run now" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("run_gpp_background_refresh_once");
    });
    expect(await screen.findByText("5 refreshed manifests")).toBeInTheDocument();
  });

  it("formats machine timestamps before displaying them", async () => {
    const rawSeedGenerated = "2026-07-03T03:36:47.140472378+00:00";
    const rawLastChecked = "2026-07-03T03:37:12.999999999+00:00";
    const rawStarted = "2026-07-03T03:38:01.000000001+00:00";
    const rawCompleted = "2026-07-03T03:39:02.000000002+00:00";

    invokeMock.mockImplementation((command: string) => {
      if (command === "app_status") {
        return Promise.resolve("SpectrumPilot desktop shell is ready");
      }
      if (command === "runtime_paths") {
        return Promise.resolve(runtimePathsMock());
      }
      if (command === "gpp_catalog_status") {
        return Promise.resolve(
          catalogStatusMock({
            seedGeneratedAt: rawSeedGenerated,
            backgroundRefreshLastStartedAt: rawStarted,
            backgroundRefreshLastCompletedAt: rawCompleted,
            lastCheckedAt: rawLastChecked,
          }),
        );
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<SettingsPage />);

    expect(await screen.findAllByText(/2026-07-03 \d{2}:\d{2}:\d{2}/)).toHaveLength(4);
    expect(screen.queryByText(rawSeedGenerated)).not.toBeInTheDocument();
    expect(screen.queryByText(rawLastChecked)).not.toBeInTheDocument();
    expect(screen.queryByText(rawStarted)).not.toBeInTheDocument();
    expect(screen.queryByText(rawCompleted)).not.toBeInTheDocument();
  });
});
