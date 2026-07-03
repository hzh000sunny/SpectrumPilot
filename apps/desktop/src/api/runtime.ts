import { invoke, isTauri } from "@tauri-apps/api/core";

export type RuntimePaths = {
  appStorageDir: string;
  configDir: string;
  metadataDir: string;
  internalCacheDir: string;
  logsDir: string;
  workspaceRoot: string;
  threeGppWorkspaceDir: string;
  threeGppInternalCacheDir: string;
  threeGppCatalogDir: string;
  appDataDir: string;
  appCacheDir: string;
  appLogDir: string;
  threeGppCacheDir: string;
};

export type RuntimeSnapshot = {
  status: string;
  paths: RuntimePaths;
};

const BROWSER_PREVIEW_SNAPSHOT: RuntimeSnapshot = {
  status: "SpectrumPilot browser preview",
  paths: {
    appStorageDir: "Preview only",
    configDir: "Preview only",
    metadataDir: "Preview only",
    internalCacheDir: "Preview only",
    logsDir: "Preview only",
    workspaceRoot: "Preview only",
    threeGppWorkspaceDir: "Preview only",
    threeGppInternalCacheDir: "Preview only",
    threeGppCatalogDir: "Preview only",
    appDataDir: "Preview only",
    appCacheDir: "Preview only",
    appLogDir: "Preview only",
    threeGppCacheDir: "Preview only",
  },
};

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  if (!isTauri()) {
    return BROWSER_PREVIEW_SNAPSHOT;
  }

  const [status, paths] = await Promise.all([
    invoke<string>("app_status"),
    invoke<RuntimePaths>("runtime_paths"),
  ]);

  return { status, paths };
}

export async function setWorkspaceRoot(workspaceRoot: string): Promise<RuntimePaths> {
  if (!isTauri()) {
    return {
      ...BROWSER_PREVIEW_SNAPSHOT.paths,
      workspaceRoot,
      threeGppWorkspaceDir: `${workspaceRoot.replace(/[\\/]$/, "")}/3gpp`,
    };
  }

  return invoke<RuntimePaths>("set_workspace_root", { workspaceRoot });
}
