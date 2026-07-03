import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type GppCatalogStatus = {
  catalogRoot: string;
  manifestCount: number;
  recordCount: number;
  indexCount: number;
  seedVersion: string;
  seedGeneratedAt: string | null;
  seedScope: string;
  backgroundRefreshEnabled: boolean;
  backgroundRefreshIntervalMinutes: number;
  backgroundRefreshTrackedRoots: number;
  backgroundRefreshMeetingWindow: number;
  backgroundRefreshState: "not_started" | "running" | "succeeded" | "failed" | "disabled";
  backgroundRefreshLastStartedAt: string | null;
  backgroundRefreshLastCompletedAt: string | null;
  backgroundRefreshLastError: string | null;
  backgroundRefreshLastRefreshedManifestCount: number;
  backgroundRefreshLogPath: string;
  lastCheckedAt: string | null;
};

export type GppBootstrapReport = {
  fetchedUrlCount: number;
  manifestCount: number;
  childEntryCount: number;
  targetRoots: string[];
  checkedAt: string;
  catalogRoot: string;
};

export type GppTdocFileResult = {
  tdoc: string;
  fileName: string;
  url: string;
  source: string;
  root: string;
  workGroup: string | null;
  meeting: string | null;
  remoteModifiedRaw: string | null;
  sizeRaw: string | null;
  sizeBytes: number | null;
};

export type GppTdocSearchReport = {
  query: string;
  normalizedQuery: string;
  source: string;
  searchedUrlCount: number;
  results: GppTdocFileResult[];
  message: string;
};

export type GppTdocDownloadReport = {
  fileName: string;
  sourceUrl: string;
  savedPath: string;
  sizeBytes: number;
};

export type GppLookupMode = "auto" | "specification" | "proposal";

export type GppLookupJobRequest = {
  query: string;
  mode: GppLookupMode;
  workGroup: string | null;
  meetingHint: string | null;
  searchWindow: "fast-recent" | "from-meeting" | "deep-search";
  openAfterDownload: boolean;
};

export type GppLookupJobStarted = {
  jobId: string;
};

export type GppLookupProgress = {
  jobId: string;
  stage: string;
  message: string;
  progress: number | null;
  searchedUrlCount: number;
};

export type GppLookupComplete = {
  jobId: string;
  query: string;
  sourceUrl: string;
  zipPath: string;
  extractedPath: string;
  openedPath: string | null;
  cacheStatus: "cached_document" | "cached_zip" | "downloaded";
  message: string;
};

const PREVIEW_STATUS: GppCatalogStatus = {
  catalogRoot: "Preview only",
  manifestCount: 0,
  recordCount: 0,
  indexCount: 0,
  seedVersion: "browser-preview",
  seedGeneratedAt: null,
  seedScope: "Browser preview does not install a 3GPP catalog seed.",
  backgroundRefreshEnabled: false,
  backgroundRefreshIntervalMinutes: 60,
  backgroundRefreshTrackedRoots: 0,
  backgroundRefreshMeetingWindow: 0,
  backgroundRefreshState: "not_started",
  backgroundRefreshLastStartedAt: null,
  backgroundRefreshLastCompletedAt: null,
  backgroundRefreshLastError: null,
  backgroundRefreshLastRefreshedManifestCount: 0,
  backgroundRefreshLogPath: "Preview only",
  lastCheckedAt: null,
};

export async function getGppCatalogStatus(): Promise<GppCatalogStatus> {
  if (!isTauri()) {
    return PREVIEW_STATUS;
  }

  return invoke<GppCatalogStatus>("gpp_catalog_status");
}

export async function setGppBackgroundRefreshEnabled(
  enabled: boolean,
): Promise<GppCatalogStatus> {
  if (!isTauri()) {
    return {
      ...PREVIEW_STATUS,
      backgroundRefreshEnabled: enabled,
      backgroundRefreshState: enabled ? "not_started" : "disabled",
    };
  }

  return invoke<GppCatalogStatus>("set_gpp_background_refresh_enabled", { enabled });
}

export async function bootstrapGppCatalog(): Promise<GppBootstrapReport> {
  if (!isTauri()) {
    return {
      fetchedUrlCount: 0,
      manifestCount: 0,
      childEntryCount: 0,
      targetRoots: [],
      checkedAt: "Preview only",
      catalogRoot: "Preview only",
    };
  }

  return invoke<GppBootstrapReport>("bootstrap_gpp_catalog");
}

export async function searchGppTdoc(query: string): Promise<GppTdocSearchReport> {
  if (!isTauri()) {
    return {
      query,
      normalizedQuery: query.trim().toUpperCase().replace(/\.ZIP$/i, ""),
      source: "browser-preview",
      searchedUrlCount: 0,
      message: "Browser preview uses sample 3GPP data.",
      results: query.trim()
        ? [
            {
              tdoc: "R2-2601401",
              fileName: "R2-2601401.zip",
              url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
              source: "browser-preview",
              root: "tsg_ran",
              workGroup: "RAN2",
              meeting: "TSGR2_133bis",
              remoteModifiedRaw: "2026/04/03 9:50",
              sizeRaw: "78,5 KB",
              sizeBytes: 80384,
            },
          ]
        : [],
    };
  }

  return invoke<GppTdocSearchReport>("search_gpp_tdoc", { request: { query } });
}

export async function downloadGppTdoc(url: string): Promise<GppTdocDownloadReport> {
  if (!isTauri()) {
    return {
      fileName: url.split("/").pop() ?? "download.zip",
      sourceUrl: url,
      savedPath: "Preview only",
      sizeBytes: 0,
    };
  }

  return invoke<GppTdocDownloadReport>("download_gpp_tdoc", { request: { url } });
}

export async function startGppLookupJob(
  request: GppLookupJobRequest,
): Promise<GppLookupJobStarted> {
  if (!isTauri()) {
    throw new Error("3GPP lookup requires the SpectrumPilot desktop runtime.");
  }

  return invoke<GppLookupJobStarted>("start_gpp_lookup_job", { request });
}

export function canStartGppLookupJob(): boolean {
  return isTauri();
}

export async function cancelGppLookupJob(jobId: string): Promise<boolean> {
  if (!isTauri()) {
    return true;
  }

  return invoke<boolean>("cancel_gpp_lookup_job", { jobId });
}

export async function listenGppLookupProgress(
  handler: (event: GppLookupProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<GppLookupProgress>("gpp-job-progress", (event) => handler(event.payload));
}

export async function listenGppLookupComplete(
  handler: (event: GppLookupComplete) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<GppLookupComplete>("gpp-job-complete", (event) => handler(event.payload));
}
