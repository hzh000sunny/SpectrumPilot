import { useEffect, useState } from "react";
import { Button, Input, Switch, Tag, Typography } from "antd";

import {
  getGppRefreshLogTail,
  getGppCatalogStatus,
  runGppBackgroundRefreshOnce,
  setGppBackgroundRefreshEnabled,
  setGppBackgroundRefreshIntervalMinutes,
  type GppCatalogStatus,
} from "../api/gppCatalog";
import { getRuntimeSnapshot, setWorkspaceRoot, type RuntimeSnapshot } from "../api/runtime";

type SettingsSectionId = "system" | "gpp";

const SETTINGS_NAV_ITEMS: Array<{ id: SettingsSectionId; label: string }> = [
  { id: "system", label: "System" },
  { id: "gpp", label: "3GPP Ftp" },
];

export function SettingsPage() {
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null);
  const [catalogStatus, setCatalogStatus] = useState<GppCatalogStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savingRefreshSetting, setSavingRefreshSetting] = useState(false);
  const [savingRefreshInterval, setSavingRefreshInterval] = useState(false);
  const [runningRefresh, setRunningRefresh] = useState(false);
  const [loadingRefreshLog, setLoadingRefreshLog] = useState(false);
  const [savingWorkspaceRoot, setSavingWorkspaceRoot] = useState(false);
  const [workspaceRootDraft, setWorkspaceRootDraft] = useState("");
  const [refreshIntervalDraft, setRefreshIntervalDraft] = useState("60");
  const [refreshLogLines, setRefreshLogLines] = useState<string[]>([]);
  const [activeSection, setActiveSection] = useState<SettingsSectionId>("gpp");

  useEffect(() => {
    let cancelled = false;

    Promise.all([getRuntimeSnapshot(), getGppCatalogStatus()])
      .then(([nextSnapshot, nextCatalogStatus]) => {
        if (!cancelled) {
          setSnapshot(nextSnapshot);
          setCatalogStatus(nextCatalogStatus);
        }
      })
      .catch((source: unknown) => {
        if (!cancelled) {
          setError(source instanceof Error ? source.message : String(source));
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (snapshot?.paths.workspaceRoot) {
      setWorkspaceRootDraft(snapshot.paths.workspaceRoot);
    }
  }, [snapshot?.paths.workspaceRoot]);

  useEffect(() => {
    if (catalogStatus?.backgroundRefreshIntervalMinutes) {
      setRefreshIntervalDraft(String(catalogStatus.backgroundRefreshIntervalMinutes));
    }
  }, [catalogStatus?.backgroundRefreshIntervalMinutes]);

  const handleScheduledUpdateChange = async (enabled: boolean) => {
    const previousStatus = catalogStatus;
    setError(null);
    setSavingRefreshSetting(true);
    if (previousStatus) {
      setCatalogStatus({
        ...previousStatus,
        backgroundRefreshEnabled: enabled,
        backgroundRefreshState: enabled
          ? previousStatus.backgroundRefreshState === "disabled"
            ? "not_started"
            : previousStatus.backgroundRefreshState
          : "disabled",
      });
    }
    try {
      const nextStatus = await setGppBackgroundRefreshEnabled(enabled);
      setCatalogStatus(nextStatus);
    } catch (source: unknown) {
      if (previousStatus) {
        setCatalogStatus(previousStatus);
      }
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setSavingRefreshSetting(false);
    }
  };

  const handleWorkspaceRootSave = async () => {
    setError(null);
    setSavingWorkspaceRoot(true);
    try {
      const paths = await setWorkspaceRoot(workspaceRootDraft);
      setSnapshot((previous) => ({
        status: previous?.status ?? "SpectrumPilot desktop shell is ready",
        paths,
      }));
    } catch (source: unknown) {
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setSavingWorkspaceRoot(false);
    }
  };

  const handleRefreshIntervalSave = async () => {
    const intervalMinutes = Number(refreshIntervalDraft);
    if (!Number.isFinite(intervalMinutes) || intervalMinutes < 5) {
      setError("Refresh interval must be at least 5 minutes.");
      return;
    }
    setError(null);
    setSavingRefreshInterval(true);
    try {
      const nextStatus = await setGppBackgroundRefreshIntervalMinutes(intervalMinutes);
      setCatalogStatus(nextStatus);
    } catch (source: unknown) {
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setSavingRefreshInterval(false);
    }
  };

  const handleManualRefresh = async () => {
    setError(null);
    setRunningRefresh(true);
    try {
      const nextStatus = await runGppBackgroundRefreshOnce();
      setCatalogStatus(nextStatus);
    } catch (source: unknown) {
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setRunningRefresh(false);
    }
  };

  const handleLoadRefreshLog = async () => {
    setError(null);
    setLoadingRefreshLog(true);
    try {
      setRefreshLogLines(await getGppRefreshLogTail(80));
    } catch (source: unknown) {
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setLoadingRefreshLog(false);
    }
  };

  return (
    <div className="settings-page">
      <div className="settings-layout">
        <nav className="settings-section-nav" aria-label="Settings sections">
          {SETTINGS_NAV_ITEMS.map((item) => (
            <button
              aria-current={activeSection === item.id ? "page" : undefined}
              className={`settings-nav-item${activeSection === item.id ? " is-active" : ""}`}
              key={item.id}
              onClick={() => setActiveSection(item.id)}
              type="button"
            >
              {item.label}
            </button>
          ))}
        </nav>

        <main className="settings-content">
          {activeSection === "system" && (
            <SystemSettingsContent
              onWorkspaceRootChange={setWorkspaceRootDraft}
              onWorkspaceRootSave={handleWorkspaceRootSave}
              savingWorkspaceRoot={savingWorkspaceRoot}
              snapshot={snapshot}
              workspaceRootDraft={workspaceRootDraft}
            />
          )}
          {activeSection === "gpp" && (
            <GppSettingsContent
              catalogStatus={catalogStatus}
              loadingRefreshLog={loadingRefreshLog}
              onLoadRefreshLog={handleLoadRefreshLog}
              onManualRefresh={handleManualRefresh}
              onRefreshIntervalChange={setRefreshIntervalDraft}
              onRefreshIntervalSave={handleRefreshIntervalSave}
              onScheduledUpdateChange={handleScheduledUpdateChange}
              refreshIntervalDraft={refreshIntervalDraft}
              refreshLogLines={refreshLogLines}
              runningRefresh={runningRefresh}
              savingRefreshInterval={savingRefreshInterval}
              savingRefreshSetting={savingRefreshSetting}
              snapshot={snapshot}
            />
          )}
        </main>
      </div>

      {error && <p className="error-text">{error}</p>}
    </div>
  );
}

function SystemSettingsContent({
  onWorkspaceRootChange,
  onWorkspaceRootSave,
  savingWorkspaceRoot,
  snapshot,
  workspaceRootDraft,
}: {
  onWorkspaceRootChange: (value: string) => void;
  onWorkspaceRootSave: () => Promise<void>;
  savingWorkspaceRoot: boolean;
  snapshot: RuntimeSnapshot | null;
  workspaceRootDraft: string;
}) {
  return (
    <>
      <header className="settings-content-header">
        <Typography.Title level={4}>System</Typography.Title>
        <Typography.Paragraph className="muted">
          Runtime status, user workspace, and internal application storage.
        </Typography.Paragraph>
      </header>

      <section className="settings-panel">
        <div className="settings-panel-header">
          <div>
            <Typography.Title level={5} className="section-title">
              Runtime
            </Typography.Title>
            <Typography.Paragraph className="muted settings-panel-copy">
              Current desktop bridge state.
            </Typography.Paragraph>
          </div>
        </div>

        <div className="settings-table path-table">
          <PathRow label="Status" value={snapshot?.status ?? "Loading"} />
        </div>
      </section>

      <section className="settings-panel">
        <div className="settings-panel-header">
          <div>
            <Typography.Title level={5} className="section-title">
              Workspace
            </Typography.Title>
            <Typography.Paragraph className="muted settings-panel-copy">
              User-visible documents, downloads, and generated files.
            </Typography.Paragraph>
          </div>
        </div>

        <div className="settings-table path-table">
          <PathRow label="Workspace root" value={snapshot?.paths.workspaceRoot ?? "Loading"} />
          <PathRow label="3GPP documents" value={snapshot?.paths.threeGppWorkspaceDir ?? "Loading"} />
        </div>

        <div className="workspace-editor">
          <Input
            aria-label="Workspace root"
            value={workspaceRootDraft}
            onChange={(event) => onWorkspaceRootChange(event.target.value)}
          />
          <Button loading={savingWorkspaceRoot} onClick={onWorkspaceRootSave} type="primary">
            Save
          </Button>
        </div>
      </section>

      <section className="settings-panel">
        <div className="settings-panel-header">
          <div>
            <Typography.Title level={5} className="section-title">
              Application Storage
            </Typography.Title>
            <Typography.Paragraph className="muted settings-panel-copy">
              Internal metadata, cache, configuration, and logs managed by SpectrumPilot.
            </Typography.Paragraph>
          </div>
        </div>

        <div className="settings-table path-table">
          <PathRow label="Storage root" value={snapshot?.paths.appStorageDir ?? "Loading"} />
          <PathRow label="Config" value={snapshot?.paths.configDir ?? "Loading"} />
          <PathRow label="Metadata" value={snapshot?.paths.metadataDir ?? "Loading"} />
          <PathRow label="Internal cache" value={snapshot?.paths.internalCacheDir ?? "Loading"} />
          <PathRow label="Logs" value={snapshot?.paths.logsDir ?? "Loading"} />
        </div>
      </section>
    </>
  );
}

function GppSettingsContent({
  snapshot,
  catalogStatus,
  loadingRefreshLog,
  onLoadRefreshLog,
  onManualRefresh,
  onRefreshIntervalChange,
  onRefreshIntervalSave,
  savingRefreshSetting,
  onScheduledUpdateChange,
  refreshIntervalDraft,
  refreshLogLines,
  runningRefresh,
  savingRefreshInterval,
}: {
  snapshot: RuntimeSnapshot | null;
  catalogStatus: GppCatalogStatus | null;
  loadingRefreshLog: boolean;
  onLoadRefreshLog: () => Promise<void>;
  onManualRefresh: () => Promise<void>;
  onRefreshIntervalChange: (value: string) => void;
  onRefreshIntervalSave: () => Promise<void>;
  savingRefreshSetting: boolean;
  onScheduledUpdateChange: (enabled: boolean) => Promise<void>;
  refreshIntervalDraft: string;
  refreshLogLines: string[];
  runningRefresh: boolean;
  savingRefreshInterval: boolean;
}) {
  return (
    <>
      <header className="settings-content-header">
        <Typography.Title level={4}>3GPP Ftp</Typography.Title>
        <Typography.Paragraph className="muted">
          Catalog refresh, internal metadata, and user documents.
        </Typography.Paragraph>
      </header>

      <section className="settings-panel scheduled-panel">
        <div className="settings-panel-header">
          <div>
            <Typography.Title level={5} className="section-title">
              Scheduled Update
            </Typography.Title>
            <Typography.Paragraph className="muted settings-panel-copy">
              Background 3GPP catalog refresh policy and latest run status.
            </Typography.Paragraph>
          </div>
          <label className="scheduled-toggle">
            <span>Enable scheduled update</span>
            <Switch
              aria-label="Enable scheduled update"
              checked={catalogStatus?.backgroundRefreshEnabled ?? false}
              loading={savingRefreshSetting}
              onChange={onScheduledUpdateChange}
            />
          </label>
        </div>

        <div className="scheduled-layout">
          <div className="status-panel">
            <span className="settings-label">Current state</span>
            <StatusTag status={effectiveRefreshState(catalogStatus)} />
            <strong>{backgroundRefreshLabel(catalogStatus)}</strong>
            <span className="settings-subtle">{refreshCountLabel(catalogStatus)}</span>
          </div>

          <div className="settings-detail-grid">
            <DetailItem label="Interval" value={intervalLabel(catalogStatus)} />
            <DetailItem label="Tracked roots" value={trackedRootsLabel(catalogStatus)} />
            <DetailItem label="Meeting window" value={meetingWindowLabel(catalogStatus)} />
            <DetailItem
              label="Last refresh started"
              value={formatDisplayTimestamp(catalogStatus?.backgroundRefreshLastStartedAt, "Never")}
              title={catalogStatus?.backgroundRefreshLastStartedAt ?? undefined}
            />
            <DetailItem
              label="Last refresh completed"
              value={formatDisplayTimestamp(
                catalogStatus?.backgroundRefreshLastCompletedAt,
                "Not completed",
              )}
              title={catalogStatus?.backgroundRefreshLastCompletedAt ?? undefined}
            />
            <DetailItem
              label="Last refresh error"
              value={catalogStatus?.backgroundRefreshLastError ?? "None"}
              tone={catalogStatus?.backgroundRefreshLastError ? "danger" : "normal"}
            />
          </div>
        </div>

        <div className="scheduled-controls">
          <label className="interval-editor">
            <span className="settings-label">Refresh interval</span>
            <Input
              aria-label="Refresh interval minutes"
              min={5}
              onChange={(event) => onRefreshIntervalChange(event.target.value)}
              type="number"
              value={refreshIntervalDraft}
            />
          </label>
          <Button loading={savingRefreshInterval} onClick={onRefreshIntervalSave}>
            Save interval
          </Button>
          <Button loading={runningRefresh} onClick={onManualRefresh} type="primary">
            Run now
          </Button>
          <Button loading={loadingRefreshLog} onClick={onLoadRefreshLog}>
            Show log
          </Button>
        </div>

        {refreshLogLines.length > 0 && (
          <pre className="refresh-log-tail" aria-label="Refresh log tail">
            {refreshLogLines.join("\n")}
          </pre>
        )}
      </section>

      <section className="settings-panel">
        <div className="settings-panel-header">
          <div>
            <Typography.Title level={5} className="section-title">
              Catalog
            </Typography.Title>
            <Typography.Paragraph className="muted settings-panel-copy">
              Local seed metadata, download state, and 3GPP index coverage.
            </Typography.Paragraph>
          </div>
          <CatalogInstallTag state={catalogStatus?.catalogInstallState} />
        </div>

        <div className="catalog-summary" aria-label="3GPP catalog status">
          <SummaryTile label="Install state" value={catalogInstallLabel(catalogStatus)} />
          <SummaryTile label="Manifests" value={`${catalogStatus?.manifestCount ?? 0} manifests`} />
          <SummaryTile
            label="Indexed TDocs"
            value={`${catalogStatus?.recordCount ?? 0} indexed TDocs`}
          />
          <SummaryTile
            label="Index shards"
            value={`${catalogStatus?.indexCount ?? 0} index shards`}
          />
        </div>

        <div className="settings-table">
          <PathRow label="Seed version" value={catalogStatus?.seedVersion ?? "Loading"} />
          <PathRow
            label="Download version"
            value={catalogStatus?.catalogDownloadVersion ?? "Not configured"}
          />
          <PathRow
            label="Download progress"
            value={catalogDownloadProgressLabel(catalogStatus)}
          />
          <PathRow
            label="Last download attempt"
            value={formatDisplayTimestamp(catalogStatus?.catalogDownloadLastAttemptAt, "Never")}
            title={catalogStatus?.catalogDownloadLastAttemptAt ?? undefined}
          />
          <PathRow
            label="Last download success"
            value={formatDisplayTimestamp(
              catalogStatus?.catalogDownloadLastSuccessAt,
              "Not completed",
            )}
            title={catalogStatus?.catalogDownloadLastSuccessAt ?? undefined}
          />
          <PathRow
            label="Last download error"
            value={catalogStatus?.catalogDownloadLastError ?? "None"}
          />
          <PathRow
            label="Seed generated"
            value={formatDisplayTimestamp(catalogStatus?.seedGeneratedAt, "Unknown")}
            title={catalogStatus?.seedGeneratedAt ?? undefined}
          />
          <PathRow label="Seed scope" value={catalogStatus?.seedScope ?? "Loading"} />
          <PathRow
            label="Last checked"
            value={formatDisplayTimestamp(catalogStatus?.lastCheckedAt, "Never")}
            title={catalogStatus?.lastCheckedAt ?? undefined}
          />
        </div>
      </section>

      <section className="settings-panel">
        <div className="settings-panel-header">
          <div>
            <Typography.Title level={5} className="section-title">
              Data Locations
            </Typography.Title>
            <Typography.Paragraph className="muted settings-panel-copy">
              User documents stay in the workspace; catalog metadata is managed internally.
            </Typography.Paragraph>
          </div>
        </div>

        <div className="settings-table path-table">
          <PathRow label="User documents" value={snapshot?.paths.threeGppWorkspaceDir ?? "Loading"} />
          <PathRow
            label="Internal catalog metadata"
            value={catalogStatus?.catalogRoot ?? snapshot?.paths.threeGppCatalogDir ?? "Loading"}
          />
          <PathRow
            label="Internal cache"
            value={snapshot?.paths.threeGppInternalCacheDir ?? "Loading"}
          />
          <PathRow label="Refresh log" value={catalogStatus?.backgroundRefreshLogPath ?? "Loading"} />
        </div>
      </section>

    </>
  );
}

function backgroundRefreshLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  if (!catalogStatus.backgroundRefreshEnabled) {
    return "Disabled";
  }
  return `Enabled, every ${catalogStatus.backgroundRefreshIntervalMinutes} minutes`;
}

function intervalLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  return `Every ${catalogStatus.backgroundRefreshIntervalMinutes} minutes`;
}

function trackedRootsLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  return `${catalogStatus.backgroundRefreshTrackedRoots} tracked roots`;
}

function meetingWindowLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  return `${catalogStatus.backgroundRefreshMeetingWindow} recent meetings per changed workgroup`;
}

function refreshStateLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  switch (catalogStatus.backgroundRefreshState) {
    case "not_started":
      return "Not started";
    case "running":
      return "Running";
    case "succeeded":
      return "Succeeded";
    case "failed":
      return "Failed";
    case "disabled":
      return "Disabled";
  }
}

function effectiveRefreshState(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return undefined;
  }
  if (!catalogStatus.backgroundRefreshEnabled) {
    return "disabled";
  }
  return catalogStatus.backgroundRefreshState;
}

function refreshCountLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  return `${catalogStatus.backgroundRefreshLastRefreshedManifestCount} refreshed manifests`;
}

function catalogInstallLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  switch (catalogStatus.catalogInstallState) {
    case "ready":
      return "Ready";
    case "downloading":
      return "Downloading";
    case "failed":
      return "Failed";
    case "not_installed":
      return "Not installed";
    default:
      return catalogStatus.catalogInstallState;
  }
}

function catalogDownloadProgressLabel(catalogStatus: GppCatalogStatus | null) {
  if (!catalogStatus) {
    return "Loading";
  }
  const downloaded = catalogStatus.catalogDownloadedBytes;
  const expected = catalogStatus.catalogDownloadExpectedBytes;
  if (downloaded == null && expected == null) {
    return "No active download";
  }
  if (downloaded != null && expected != null && expected > 0) {
    return `${formatBytes(downloaded)} / ${formatBytes(expected)}`;
  }
  if (downloaded != null) {
    return `${formatBytes(downloaded)} downloaded`;
  }
  return `${formatBytes(expected ?? 0)} expected`;
}

function formatBytes(bytes: number) {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

function formatDisplayTimestamp(value: string | null | undefined, fallback: string) {
  if (!value) {
    return fallback;
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const parts = new Intl.DateTimeFormat("en-CA", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).formatToParts(date);
  const part = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find((item) => item.type === type)?.value ?? "";

  return `${part("year")}-${part("month")}-${part("day")} ${part("hour")}:${part("minute")}:${part("second")}`;
}

function StatusTag({ status }: { status: GppCatalogStatus["backgroundRefreshState"] | undefined }) {
  const label = status
    ? refreshStateLabel({ backgroundRefreshState: status } as GppCatalogStatus)
    : "Loading";
  const color =
    status === "succeeded"
      ? "success"
      : status === "running"
        ? "processing"
        : status === "failed"
          ? "error"
          : status === "disabled"
            ? "default"
            : "warning";

  return (
    <Tag color={color} className="status-tag">
      {label}
    </Tag>
  );
}

function CatalogInstallTag({ state }: { state: GppCatalogStatus["catalogInstallState"] | undefined }) {
  const label = state ? catalogInstallLabel({ catalogInstallState: state } as GppCatalogStatus) : "Loading";
  const color =
    state === "ready"
      ? "green"
      : state === "downloading"
        ? "blue"
        : state === "failed"
          ? "red"
          : "default";
  return (
    <Tag color={color} className="status-tag">
      {label}
    </Tag>
  );
}

function DetailItem({
  label,
  value,
  title,
  tone = "normal",
}: {
  label: string;
  value: string;
  title?: string;
  tone?: "normal" | "danger";
}) {
  return (
    <div className="settings-detail-item">
      <span>{label}</span>
      <strong className={tone === "danger" ? "danger-value" : undefined} title={title}>
        {value}
      </strong>
    </div>
  );
}

function SummaryTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="summary-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function PathRow({ label, value, title }: { label: string; value: string; title?: string }) {
  return (
    <div className="settings-row">
      <span>{label}</span>
      <code title={title ?? value}>{value}</code>
    </div>
  );
}
