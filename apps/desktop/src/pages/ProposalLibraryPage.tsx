import { useEffect, useState } from "react";
import { Alert, Tag, Typography } from "antd";

import { getGppLookupHistory, type LookupHistoryRecord } from "../api/gppCatalog";

export function ProposalLibraryPage() {
  const [records, setRecords] = useState<LookupHistoryRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getGppLookupHistory(100)
      .then((nextRecords) => {
        if (!cancelled) {
          setRecords(nextRecords);
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

  return (
    <section className="section library-section">
      <div className="section-heading">
        <Typography.Title level={5} className="section-title">
          Proposal Library
        </Typography.Title>
        <span>{records.length} local records</span>
      </div>

      {error && <Alert className="lookup-alert" type="error" showIcon title={error} />}

      {records.length === 0 && !error ? (
        <div className="empty-panel">No lookup history yet.</div>
      ) : (
        <div className="library-table" aria-label="3GPP lookup history">
          <div className="library-row library-head">
            <span>Query</span>
            <span>Cache</span>
            <span>Completed</span>
            <span>Source URL</span>
            <span>ZIP path</span>
            <span>Opened path</span>
          </div>
          {records.map((record) => (
            <div className="library-row" key={`${record.completedAt}-${record.query}`}>
              <code>{record.query}</code>
              <CacheStatusTag status={record.cacheStatus} />
              <span title={record.completedAt}>{formatDisplayTimestamp(record.completedAt)}</span>
              <code>{record.sourceUrl}</code>
              <code>{record.zipPath}</code>
              <code>{record.openedPath ?? "Not opened"}</code>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function CacheStatusTag({ status }: { status: string }) {
  const label = cacheStatusLabel(status);
  const color = status === "downloaded" ? "blue" : status === "cached_document" ? "success" : "default";

  return (
    <Tag color={color} className="status-tag">
      {label}
    </Tag>
  );
}

function cacheStatusLabel(status: string) {
  switch (status) {
    case "cached_document":
      return "Opened cached document";
    case "cached_zip":
      return "Extracted local ZIP";
    case "downloaded":
      return "Downloaded from 3GPP FTP";
    default:
      return status;
  }
}

function formatDisplayTimestamp(value: string) {
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
