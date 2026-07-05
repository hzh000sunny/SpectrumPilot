import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  CheckCircleOutlined,
  CloudDownloadOutlined,
  DatabaseOutlined,
  SearchOutlined,
  StopOutlined,
} from "@ant-design/icons";
import {
  Alert,
  Button,
  Collapse,
  Input,
  Modal,
  Progress,
  Segmented,
  Select,
  Space,
  Steps,
  Switch,
  Tag,
  Typography,
} from "antd";

import {
  cancelGppLookupJob,
  canStartGppLookupJob,
  continueGppLookupWithCandidate,
  getGppCatalogStatus,
  listenGppLookupCandidates,
  listenGppLookupComplete,
  listenGppLookupProgress,
  startGppLookupJob,
  type GppCatalogStatus,
  type GppLookupCandidate,
  type GppLookupCandidates,
  type GppLookupComplete,
  type GppLookupMode,
  type GppLookupProgress,
} from "../api/gppCatalog";

type LookupStage = "idle" | "starting" | "resolving" | "downloading" | "extracting" | "opening" | "complete" | "error" | "cancelled";
type SearchWindow = "fast-recent" | "from-meeting" | "deep-search";
type BatchLookupState = "pending" | "running" | "done" | "error" | "cancelled";

type ProgressState = {
  stage: LookupStage;
  message: string;
  percent: number;
  searchedUrlCount: number;
};

type BatchLookupItem = {
  id: string;
  query: string;
  state: BatchLookupState;
  message: string;
  jobId?: string;
};

const INITIAL_PROGRESS: ProgressState = {
  stage: "idle",
  message: "Starting lookup...",
  percent: 6,
  searchedUrlCount: 0,
};

const WORK_GROUP_OPTIONS = [
  { label: "Auto detect", value: "" },
  { label: "RAN", value: "RAN" },
  { label: "RAN1", value: "RAN1" },
  { label: "RAN2", value: "RAN2" },
  { label: "RAN3", value: "RAN3" },
  { label: "RAN4", value: "RAN4" },
  { label: "RAN5", value: "RAN5" },
  { label: "SA", value: "SA" },
  { label: "SA1", value: "SA1" },
  { label: "SA2", value: "SA2" },
  { label: "SA3", value: "SA3" },
  { label: "SA4", value: "SA4" },
  { label: "SA5", value: "SA5" },
  { label: "SA6", value: "SA6" },
  { label: "CT", value: "CT" },
  { label: "CT1", value: "CT1" },
  { label: "CT2", value: "CT2" },
  { label: "CT3", value: "CT3" },
  { label: "CT4", value: "CT4" },
  { label: "CT5", value: "CT5" },
  { label: "CT6", value: "CT6" },
];

const SEARCH_WINDOW_OPTIONS: Array<{ label: string; value: SearchWindow }> = [
  { label: "Fast recent", value: "fast-recent" },
  { label: "From meeting", value: "from-meeting" },
  { label: "Deep search", value: "deep-search" },
];

export function GppPage() {
  const [catalogStatus, setCatalogStatus] = useState<GppCatalogStatus | null>(null);
  const [lookupMode, setLookupMode] = useState<GppLookupMode>("auto");
  const [query, setQuery] = useState("");
  const [workGroup, setWorkGroup] = useState("");
  const [meetingHint, setMeetingHint] = useState("");
  const [searchWindow, setSearchWindow] = useState<SearchWindow>("fast-recent");
  const [openAfterDownload, setOpenAfterDownload] = useState(true);
  const [batchInput, setBatchInput] = useState("");
  const [batchItems, setBatchItems] = useState<BatchLookupItem[]>([]);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [progressModalOpen, setProgressModalOpen] = useState(false);
  const [progress, setProgress] = useState<ProgressState>(INITIAL_PROGRESS);
  const [lastComplete, setLastComplete] = useState<GppLookupComplete | null>(null);
  const [candidatePayload, setCandidatePayload] = useState<GppLookupCandidates | null>(null);
  const [lookupError, setLookupError] = useState<string | null>(null);
  const activeJobIdRef = useRef<string | null>(null);
  const acceptingProgressRef = useRef(false);
  const batchItemsRef = useRef<BatchLookupItem[]>([]);
  const batchRunningRef = useRef(false);
  const activeBatchItemIdRef = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getGppCatalogStatus()
      .then((nextCatalogStatus) => {
        if (!cancelled) {
          setCatalogStatus(nextCatalogStatus);
        }
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let progressUnlisten: (() => void) | undefined;
    let completeUnlisten: (() => void) | undefined;
    let candidatesUnlisten: (() => void) | undefined;
    let disposed = false;

    listenGppLookupProgress((event) => {
      if (!acceptJobEvent(event.jobId)) {
        return;
      }
      setProgress(progressFromEvent(event));
      if (activeBatchItemIdRef.current) {
        updateBatchItem(activeBatchItemIdRef.current, {
          jobId: event.jobId,
          state: event.stage === "error" ? "error" : event.stage === "cancelled" ? "cancelled" : "running",
          message: event.message,
        });
        if (event.stage === "error" || event.stage === "cancelled") {
          setCandidatePayload(null);
        }
        if (event.stage === "error" || event.stage === "cancelled") {
          finishActiveBatchItem();
          setProgressModalOpen(false);
          void startNextBatchItem();
        }
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        progressUnlisten = unlisten;
      }
    });

    listenGppLookupComplete((event) => {
      if (!acceptJobEvent(event.jobId)) {
        return;
      }
      if (activeBatchItemIdRef.current) {
        updateBatchItem(activeBatchItemIdRef.current, {
          jobId: event.jobId,
          state: "done",
          message: event.message,
        });
        setLastComplete(event);
        setCandidatePayload(null);
        setLookupError(null);
        finishActiveBatchItem();
        setProgressModalOpen(false);
        getGppCatalogStatus()
          .then(setCatalogStatus)
          .catch(() => undefined);
        void startNextBatchItem();
        return;
      }
      setProgress({
        stage: "complete",
        message: event.message,
        percent: 100,
        searchedUrlCount: 0,
      });
      setLastComplete(event);
      setCandidatePayload(null);
      setLookupError(null);
      setActiveJobId(null);
      activeJobIdRef.current = null;
      acceptingProgressRef.current = false;
      setProgressModalOpen(false);
      getGppCatalogStatus()
        .then(setCatalogStatus)
        .catch(() => undefined);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        completeUnlisten = unlisten;
      }
    });

    listenGppLookupCandidates((event) => {
      if (activeJobIdRef.current && activeJobIdRef.current !== event.jobId) {
        return;
      }
      activeJobIdRef.current = event.jobId;
      acceptingProgressRef.current = true;
      setActiveJobId(event.jobId);
      setCandidatePayload(event);
      setProgressModalOpen(false);
      setProgress({
        stage: "resolving",
        message: `${event.candidates.length} exact candidates found. Choose one to download.`,
        percent: 58,
        searchedUrlCount: 0,
      });
      if (activeBatchItemIdRef.current) {
        updateBatchItem(activeBatchItemIdRef.current, {
          jobId: event.jobId,
          state: "running",
          message: "Waiting for candidate selection.",
        });
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        candidatesUnlisten = unlisten;
      }
    });

    return () => {
      disposed = true;
      progressUnlisten?.();
      completeUnlisten?.();
      candidatesUnlisten?.();
    };
  }, []);

  const progressSteps = useMemo(
    () => [
      { title: "Resolve" },
      { title: "Download" },
      { title: "Extract" },
      { title: "Open" },
    ],
    [],
  );

  const currentStep = currentProgressStep(progress.stage);
  const progressStatus = progress.stage === "error" ? "exception" : progress.stage === "complete" ? "success" : "active";
  const batchHasActiveItems = batchItems.some((item) => item.state === "pending" || item.state === "running");

  async function handleLookup(event?: FormEvent) {
    event?.preventDefault();
    const nextQuery = query.trim();
    if (!nextQuery) {
      setLookupError("Enter a specification number or proposal number.");
      return;
    }
    if (!canStartGppLookupJob()) {
      setLookupError("3GPP lookup requires the SpectrumPilot desktop runtime.");
      return;
    }

    setLookupError(null);
    setLastComplete(null);
    setCandidatePayload(null);
    activeJobIdRef.current = null;
    acceptingProgressRef.current = true;
    setProgress(INITIAL_PROGRESS);
    setProgressModalOpen(true);

    try {
      const started = await startGppLookupJob({
        query: nextQuery,
        mode: lookupMode,
        workGroup: workGroup || null,
        meetingHint: meetingHint.trim() || null,
        searchWindow,
        openAfterDownload,
      });
      if (!activeJobIdRef.current) {
        activeJobIdRef.current = started.jobId;
        setActiveJobId(started.jobId);
      }
    } catch (source) {
      activeJobIdRef.current = null;
      acceptingProgressRef.current = false;
      setProgressModalOpen(false);
      setProgress({
        stage: "error",
        message: messageFromError(source),
        percent: 100,
        searchedUrlCount: 0,
      });
      setLookupError(messageFromError(source));
    }
  }

  async function handleStartBatch() {
    const queries = parseBatchQueries(batchInput);
    if (queries.length === 0) {
      setLookupError("Enter one query per line for batch lookup.");
      return;
    }
    if (!canStartGppLookupJob()) {
      setLookupError("3GPP lookup requires the SpectrumPilot desktop runtime.");
      return;
    }

    const nextItems = queries.map((batchQuery, index) => ({
      id: `batch-${Date.now()}-${index}`,
      query: batchQuery,
      state: "pending" as const,
      message: "Waiting",
    }));
    setLookupError(null);
    setLastComplete(null);
    setCandidatePayload(null);
    commitBatchItems(nextItems);
    batchRunningRef.current = true;
    activeBatchItemIdRef.current = null;
    activeJobIdRef.current = null;
    setActiveJobId(null);
    await startNextBatchItem();
  }

  async function startNextBatchItem() {
    if (!batchRunningRef.current || activeBatchItemIdRef.current) {
      return;
    }
    const nextItem = batchItemsRef.current.find((item) => item.state === "pending");
    if (!nextItem) {
      batchRunningRef.current = false;
      return;
    }

    activeBatchItemIdRef.current = nextItem.id;
    activeJobIdRef.current = null;
    acceptingProgressRef.current = true;
    setActiveJobId(null);
    setProgress(INITIAL_PROGRESS);
    setProgressModalOpen(true);
    updateBatchItem(nextItem.id, {
      state: "running",
      message: "Starting lookup...",
    });

    try {
      const started = await startGppLookupJob({
        query: nextItem.query,
        mode: lookupMode,
        workGroup: workGroup || null,
        meetingHint: meetingHint.trim() || null,
        searchWindow,
        openAfterDownload,
      });
      activeJobIdRef.current = started.jobId;
      setActiveJobId(started.jobId);
      updateBatchItem(nextItem.id, {
        jobId: started.jobId,
        state: "running",
        message: "Lookup running.",
      });
    } catch (source) {
      updateBatchItem(nextItem.id, {
        state: "error",
        message: messageFromError(source),
      });
      finishActiveBatchItem();
      setProgressModalOpen(false);
      await startNextBatchItem();
    }
  }

  async function handleCancelJob() {
    const jobId = activeJobIdRef.current ?? activeJobId;
    setProgressModalOpen(false);
    setCandidatePayload(null);
    setProgress((current) => ({
      ...current,
      stage: "cancelled",
      message: "Lookup cancelled.",
    }));
    setActiveJobId(null);
    activeJobIdRef.current = null;
    acceptingProgressRef.current = false;
    if (activeBatchItemIdRef.current) {
      updateBatchItem(activeBatchItemIdRef.current, {
        state: "cancelled",
        message: "Lookup cancelled.",
        jobId: jobId ?? undefined,
      });
      activeBatchItemIdRef.current = null;
    }

    if (jobId) {
      await cancelGppLookupJob(jobId).catch(() => false);
    }
    if (batchRunningRef.current) {
      await startNextBatchItem();
    }
  }

  async function handleSelectCandidate(candidate: GppLookupCandidate) {
    if (!candidatePayload) {
      return;
    }

    try {
      await continueGppLookupWithCandidate(candidatePayload.jobId, candidate);
      setCandidatePayload(null);
      setProgress({
        stage: "resolving",
        message: "Downloading selected candidate...",
        percent: 60,
        searchedUrlCount: 0,
      });
      setProgressModalOpen(true);
    } catch (source) {
      setLookupError(messageFromError(source));
      setCandidatePayload(null);
      setProgressModalOpen(false);
      setProgress({
        stage: "error",
        message: messageFromError(source),
        percent: 100,
        searchedUrlCount: 0,
      });
    }
  }

  function commitBatchItems(items: BatchLookupItem[]) {
    batchItemsRef.current = items;
    setBatchItems(items);
  }

  function updateBatchItem(id: string, patch: Partial<BatchLookupItem>) {
    commitBatchItems(batchItemsRef.current.map((item) => (item.id === id ? { ...item, ...patch } : item)));
  }

  function finishActiveBatchItem() {
    setActiveJobId(null);
    activeJobIdRef.current = null;
    activeBatchItemIdRef.current = null;
    acceptingProgressRef.current = false;
  }

  function acceptJobEvent(jobId: string) {
    if (activeJobIdRef.current) {
      return activeJobIdRef.current === jobId;
    }
    if (!acceptingProgressRef.current) {
      return false;
    }
    activeJobIdRef.current = jobId;
    setActiveJobId(jobId);
    return true;
  }

  return (
    <div className="gpp-page">
      <section className="workbench">
        <div className="workbench-header">
          <div>
            <Typography.Title level={3} className="workbench-title">
              3GPP Ftp
            </Typography.Title>
            <Typography.Paragraph className="muted workbench-copy">
              Search the 3GPP FTP archive, then download, extract, and open specifications or proposals.
            </Typography.Paragraph>
          </div>
          <div className="index-pill">
            <DatabaseOutlined />
            <span>{catalogStatus?.recordCount ?? 0} indexed TDocs</span>
          </div>
        </div>

        <form className="lookup-form" onSubmit={handleLookup}>
          <div className="lookup-toolbar">
            <Segmented
              value={lookupMode}
              onChange={(value) => setLookupMode(value as GppLookupMode)}
              options={[
                { label: "Auto Detect", value: "auto" },
                { label: "Spec Archive", value: "specification" },
                { label: "TDoc Proposal", value: "proposal" },
              ]}
            />
            <Space size={8} className="open-toggle">
              <Switch size="small" checked={openAfterDownload} onChange={setOpenAfterDownload} />
              <span>Open after download</span>
            </Space>
          </div>

          <label className="search-label" htmlFor="gpp-query">
            Query
          </label>
          <div className="search-controls lookup-controls">
            <Input
              id="gpp-query"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="R2-2601401, R2-2601401 TSGR2_133bis, 38.321, or 38.321 f10"
              autoComplete="off"
              size="large"
            />
            <Button
              type="primary"
              htmlType="submit"
              size="large"
              icon={<SearchOutlined />}
              loading={progressModalOpen && activeJobId !== null}
            >
              Find, Download & Open
            </Button>
          </div>

          <Collapse
            className="advanced-scope"
            ghost
            items={[
              {
                key: "advanced",
                label: "Advanced Scope",
                children: (
                  <div className="advanced-grid">
                    <label>
                      <span>Work group</span>
                      <Select value={workGroup} onChange={setWorkGroup} options={WORK_GROUP_OPTIONS} />
                    </label>
                    <label>
                      <span>Meeting hint</span>
                      <Input
                        value={meetingHint}
                        onChange={(event) => setMeetingHint(event.target.value)}
                        placeholder="TSGR2_133bis or 133bis"
                      />
                    </label>
                    <label>
                      <span>Search window</span>
                      <Select value={searchWindow} onChange={setSearchWindow} options={SEARCH_WINDOW_OPTIONS} />
                    </label>
                  </div>
                ),
              },
            ]}
          />
        </form>

        <div className="batch-lookup">
          <div className="batch-header">
            <Typography.Title level={5} className="section-title">
              Batch Lookup
            </Typography.Title>
            <Button onClick={handleStartBatch} disabled={batchHasActiveItems}>
              Start batch
            </Button>
          </div>
          <label className="search-label" htmlFor="gpp-batch-queries">
            Batch queries
          </label>
          <Input.TextArea
            id="gpp-batch-queries"
            aria-label="Batch queries"
            value={batchInput}
            onChange={(event) => setBatchInput(event.target.value)}
            placeholder={"R2-2601401\n38.321\nS2-260001"}
            rows={3}
          />
          {batchItems.length > 0 && (
            <div className="batch-table" aria-label="Batch lookup queue">
              <div className="batch-row batch-head">
                <span>Query</span>
                <span>Status</span>
                <span>Message</span>
              </div>
              {batchItems.map((item) => (
                <div className="batch-row" key={item.id}>
                  <code>{item.query}</code>
                  <BatchStateTag state={item.state} />
                  <span>{item.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        {lookupError && <Alert className="lookup-alert" type="error" showIcon title={lookupError} />}
      </section>

      <section className="section lookup-rules-section">
        <div className="section-heading">
          <Typography.Title level={5} className="section-title">
            Lookup Rules
          </Typography.Title>
          <span>Auto Detect chooses the document type from the query.</span>
        </div>
        <div className="rules-grid">
          <div className="rule-item">
            <strong>Specifications</strong>
            <span>
              Use <code>38.321</code> or <code>38321</code> for latest, <code>38.321 f</code> for latest Release
              15, and <code>38.321 f10</code> for an exact archive version.
            </span>
          </div>
          <div className="rule-item">
            <strong>Proposals</strong>
            <span>
              Use <code>R2-2601401</code> for automatic RAN2 lookup, add <code>TSGR2_133bis</code> to jump to a
              meeting, or use <code>from TSGR2_120</code> to start scanning from a meeting.
            </span>
          </div>
          <div className="rule-item">
            <strong>Resolution order</strong>
            <span>
              SpectrumPilot checks the local index first, probes exact likely URLs, then falls back to targeted
              online listing scans when needed.
            </span>
          </div>
        </div>
      </section>

      {lastComplete && (
        <section className="section result-summary-section">
          <div className="section-heading">
            <Typography.Title level={5} className="section-title">
              Last Lookup
            </Typography.Title>
            <CheckCircleOutlined className="success-icon" />
          </div>
          <div className="detail-list compact-detail">
            <span>Storage action</span>
            <strong>{cacheStatusLabel(lastComplete.cacheStatus)}</strong>
            <span>Message</span>
            <strong>{lastComplete.message}</strong>
            <span>Source URL</span>
            <code>{lastComplete.sourceUrl}</code>
            <span>ZIP path</span>
            <code>{lastComplete.zipPath}</code>
            <span>Extracted path</span>
            <code>{lastComplete.extractedPath}</code>
            <span>Opened path</span>
            <code>{lastComplete.openedPath ?? "Not opened"}</code>
          </div>
        </section>
      )}

      <Modal
        title="Select 3GPP Candidate"
        open={candidatePayload !== null}
        onCancel={handleCancelJob}
        footer={null}
        width={860}
        destroyOnHidden
      >
        {candidatePayload && (
          <div className="candidate-selection">
            <div className="candidate-summary">
              <strong>{candidatePayload.query}</strong>
              <span>{candidatePayload.candidates.length} exact ZIP files matched this query.</span>
            </div>
            <div className="candidate-table" aria-label="3GPP candidate files">
              <div className="candidate-row candidate-head">
                <span>Work group</span>
                <span>Meeting</span>
                <span>URL</span>
                <span>Action</span>
              </div>
              {candidatePayload.candidates.map((candidate) => (
                <div className="candidate-row" key={`${candidate.meeting}-${candidate.sourceUrl}`}>
                  <span>{candidate.workGroup}</span>
                  <code>{candidate.meeting}</code>
                  <code>{candidate.sourceUrl}</code>
                  <Button size="small" type="primary" onClick={() => void handleSelectCandidate(candidate)}>
                    Download
                  </Button>
                </div>
              ))}
            </div>
          </div>
        )}
      </Modal>

      <Modal
        title="3GPP Lookup Progress"
        open={progressModalOpen}
        onCancel={handleCancelJob}
        footer={null}
        mask={{ closable: false }}
        destroyOnHidden
      >
        <div className="lookup-progress-body">
          <Progress percent={progress.percent} status={progressStatus} />
          <Steps current={currentStep} items={progressSteps} size="small" />
          <div className="progress-message">
            <CloudDownloadOutlined />
            <span>{progress.message}</span>
          </div>
          <div className="progress-meta">
            <span>{progress.searchedUrlCount} remote URLs checked</span>
            <span>{stageLabel(progress.stage)}</span>
          </div>
          <Button block icon={<StopOutlined />} onClick={handleCancelJob}>
            Cancel lookup
          </Button>
        </div>
      </Modal>
    </div>
  );
}

function progressFromEvent(event: GppLookupProgress): ProgressState {
  return {
    stage: normalizeStage(event.stage),
    message: event.message,
    percent: event.progress ?? progressPercentForStage(event.stage),
    searchedUrlCount: event.searchedUrlCount,
  };
}

function normalizeStage(stage: string): LookupStage {
  if (
    stage === "starting" ||
    stage === "resolving" ||
    stage === "downloading" ||
    stage === "extracting" ||
    stage === "opening" ||
    stage === "complete" ||
    stage === "error" ||
    stage === "cancelled"
  ) {
    return stage;
  }
  if (stage === "probing" || stage === "listing" || stage === "candidate") {
    return "resolving";
  }
  return "starting";
}

function progressPercentForStage(stage: string) {
  switch (normalizeStage(stage)) {
    case "resolving":
      return 28;
    case "downloading":
      return 58;
    case "extracting":
      return 78;
    case "opening":
      return 92;
    case "complete":
      return 100;
    case "error":
    case "cancelled":
      return 100;
    default:
      return 10;
  }
}

function currentProgressStep(stage: LookupStage) {
  switch (stage) {
    case "downloading":
      return 1;
    case "extracting":
      return 2;
    case "opening":
    case "complete":
      return 3;
    default:
      return 0;
  }
}

function stageLabel(stage: LookupStage) {
  switch (stage) {
    case "resolving":
      return "Resolving";
    case "downloading":
      return "Downloading";
    case "extracting":
      return "Extracting";
    case "opening":
      return "Opening";
    case "complete":
      return "Complete";
    case "error":
      return "Error";
    case "cancelled":
      return "Cancelled";
    default:
      return "Starting";
  }
}

function cacheStatusLabel(status: GppLookupComplete["cacheStatus"]) {
  switch (status) {
    case "cached_document":
      return "Opened cached document";
    case "cached_zip":
      return "Extracted local ZIP";
    case "downloaded":
      return "Downloaded from 3GPP FTP";
  }
}

function parseBatchQueries(value: string) {
  return value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function BatchStateTag({ state }: { state: BatchLookupState }) {
  const color =
    state === "done"
      ? "success"
      : state === "running"
        ? "processing"
        : state === "error"
          ? "error"
          : state === "cancelled"
            ? "default"
            : "warning";
  const label =
    state === "done"
      ? "Done"
      : state === "running"
        ? "Running"
        : state === "error"
          ? "Error"
          : state === "cancelled"
            ? "Cancelled"
            : "Pending";

  return (
    <Tag color={color} className="status-tag">
      {label}
    </Tag>
  );
}

function messageFromError(source: unknown) {
  return source instanceof Error ? source.message : String(source);
}
