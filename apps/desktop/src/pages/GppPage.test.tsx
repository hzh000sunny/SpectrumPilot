import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { GppPage } from "./GppPage";

const invokeMock = vi.hoisted(() => vi.fn());
const isTauriMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

describe("GppPage", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
    isTauriMock.mockReturnValue(true);
  });

  it("displays the search workbench without storage diagnostics", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot:
            "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\metadata\\3gpp\\catalog",
          manifestCount: 0,
          recordCount: 0,
          indexCount: 0,
          lastCheckedAt: null,
        });
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);

    expect(await screen.findByRole("heading", { name: "3GPP Ftp" })).toBeInTheDocument();
    expect(await screen.findByRole("textbox", { name: /query/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Auto Detect" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Spec Archive" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "TDoc Proposal" })).toBeInTheDocument();
    expect(await screen.findByText("0 indexed TDocs")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Lookup Rules" })).toBeInTheDocument();
    expect(screen.getByText("Auto Detect chooses the document type from the query.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /bootstrap roots/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Local Index & Diagnostics" })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Download Status" })).not.toBeInTheDocument();
    expect(screen.queryByText("Seeded cache")).not.toBeInTheDocument();
    expect(
      screen.queryByText(
        "C:\\Users\\alice\\AppData\\Roaming\\SpectrumPilot\\metadata\\3gpp\\catalog",
      ),
    ).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("app_status");
    expect(invokeMock).not.toHaveBeenCalledWith("runtime_paths");
  });

  it("starts a lookup job and shows a cancellable progress modal", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command: string) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot: "Preview only",
          manifestCount: 0,
          recordCount: 0,
          indexCount: 0,
          lastCheckedAt: null,
        });
      }
      if (command === "start_gpp_lookup_job") {
        return Promise.resolve({ jobId: "job-1" });
      }
      if (command === "cancel_gpp_lookup_job") {
        return Promise.resolve(true);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);
    await user.type(await screen.findByRole("textbox", { name: /query/i }), "R2-2601401");
    await user.click(await screen.findByRole("button", { name: /find, download & open/i }));

    expect(await screen.findByRole("dialog", { name: /3gpp lookup progress/i })).toBeInTheDocument();
    expect(screen.getByText(/starting lookup/i)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("start_gpp_lookup_job", {
      request: {
        query: "R2-2601401",
        mode: "auto",
        workGroup: null,
        meetingHint: null,
        searchWindow: "fast-recent",
        openAfterDownload: true,
      },
    });

    await user.click(screen.getByRole("button", { name: /close/i }));
    expect(invokeMock).toHaveBeenCalledWith("cancel_gpp_lookup_job", { jobId: "job-1" });
  });

  it("shows a desktop runtime error instead of starting a browser preview job", async () => {
    const user = userEvent.setup();
    isTauriMock.mockReturnValue(false);

    render(<GppPage />);
    await user.type(await screen.findByRole("textbox", { name: /query/i }), "R2-2601401");
    await user.click(await screen.findByRole("button", { name: /find, download & open/i }));

    expect(await screen.findByText(/desktop runtime/i)).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: /3gpp lookup progress/i })).not.toBeInTheDocument();
  });

  it("does not overwrite an early backend progress event when job start returns", async () => {
    const user = userEvent.setup();
    let progressHandler: ((event: { payload: unknown }) => void) | undefined;
    let resolveStart: ((value: { jobId: string }) => void) | undefined;

    listenMock.mockImplementation((event: string, handler: (event: { payload: unknown }) => void) => {
      if (event === "gpp-job-progress") {
        progressHandler = handler;
      }
      return Promise.resolve(() => undefined);
    });
    invokeMock.mockImplementation((command: string) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot: "Preview only",
          manifestCount: 0,
          recordCount: 0,
          indexCount: 0,
          lastCheckedAt: null,
        });
      }
      if (command === "start_gpp_lookup_job") {
        return new Promise((resolve) => {
          resolveStart = resolve;
        });
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);
    await user.type(await screen.findByRole("textbox", { name: /query/i }), "R2-2601401");
    await user.click(await screen.findByRole("button", { name: /find, download & open/i }));

    act(() => {
      progressHandler?.({
        payload: {
          jobId: "job-1",
          stage: "resolving",
          message: "Checking local catalog for R2-2601401...",
          progress: 18,
          searchedUrlCount: 0,
        },
      });
      resolveStart?.({ jobId: "job-1" });
    });

    expect(await screen.findByText("Checking local catalog for R2-2601401...")).toBeInTheDocument();
    expect(screen.queryByText("Starting lookup...")).not.toBeInTheDocument();
  });

  it("shows whether the completed lookup used a local cached document", async () => {
    const user = userEvent.setup();
    let completeHandler: ((event: { payload: unknown }) => void) | undefined;

    listenMock.mockImplementation((event: string, handler: (event: { payload: unknown }) => void) => {
      if (event === "gpp-job-complete") {
        completeHandler = handler;
      }
      return Promise.resolve(() => undefined);
    });
    invokeMock.mockImplementation((command: string) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot: "Preview only",
          manifestCount: 7,
          recordCount: 2696,
          indexCount: 2,
          seedVersion: "stage-seed-2026-07-02",
          seedGeneratedAt: "2026-07-02T00:00:00Z",
          seedScope: "RAN2 meetings TSGR2_132 and TSGR2_133bis",
          lastCheckedAt: "2026-07-02T00:00:00Z",
        });
      }
      if (command === "start_gpp_lookup_job") {
        return Promise.resolve({ jobId: "job-1" });
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);
    await user.type(await screen.findByRole("textbox", { name: /query/i }), "R2-2601401");
    await user.click(await screen.findByRole("button", { name: /find, download & open/i }));

    act(() => {
      completeHandler?.({
        payload: {
          jobId: "job-1",
          query: "R2-2601401",
          sourceUrl: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
          zipPath: "C:\\Users\\alice\\SpectrumPilotWorkspace\\3gpp\\tdocs\\RAN2\\TSGR2_133bis\\R2-2601401\\R2-2601401.zip",
          extractedPath: "C:\\Users\\alice\\SpectrumPilotWorkspace\\3gpp\\tdocs\\RAN2\\TSGR2_133bis\\R2-2601401",
          openedPath: "C:\\Users\\alice\\SpectrumPilotWorkspace\\3gpp\\tdocs\\RAN2\\TSGR2_133bis\\R2-2601401\\R2-2601401.docx",
          cacheStatus: "cached_document",
          message: "Opened cached R2-2601401.",
        },
      });
    });

    expect(await screen.findByRole("heading", { name: "Last Lookup" })).toBeInTheDocument();
    expect(screen.getByText("Storage action")).toBeInTheDocument();
    expect(screen.getByText("Opened cached document")).toBeInTheDocument();
    expect(screen.getByText("Opened cached R2-2601401.")).toBeInTheDocument();
  });

  it("runs batch queries sequentially and marks pending, running, done, and error rows", async () => {
    const user = userEvent.setup();
    let progressHandler: ((event: { payload: unknown }) => void) | undefined;
    let completeHandler: ((event: { payload: unknown }) => void) | undefined;
    const startedQueries: string[] = [];

    listenMock.mockImplementation((event: string, handler: (event: { payload: unknown }) => void) => {
      if (event === "gpp-job-progress") {
        progressHandler = handler;
      }
      if (event === "gpp-job-complete") {
        completeHandler = handler;
      }
      return Promise.resolve(() => undefined);
    });
    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot: "Preview only",
          manifestCount: 0,
          recordCount: 0,
          indexCount: 0,
          lastCheckedAt: null,
        });
      }
      if (command === "start_gpp_lookup_job") {
        const request = (args as { request: { query: string } }).request;
        startedQueries.push(request.query);
        return Promise.resolve({ jobId: `job-${startedQueries.length}` });
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);

    await user.type(
      await screen.findByRole("textbox", { name: "Batch queries" }),
      "R2-2601401\n38.321\nS2-260001",
    );
    await user.click(screen.getByRole("button", { name: "Start batch" }));

    const queue = await screen.findByLabelText("Batch lookup queue");
    expect(within(queue).getByText("R2-2601401")).toBeInTheDocument();
    expect(within(queue).getByText("38.321")).toBeInTheDocument();
    expect(within(queue).getByText("S2-260001")).toBeInTheDocument();
    expect(within(queue).getByText("Running")).toBeInTheDocument();
    expect(within(queue).getAllByText("Pending")).toHaveLength(2);
    expect(startedQueries).toEqual(["R2-2601401"]);

    act(() => {
      completeHandler?.({
        payload: lookupCompletePayload("job-1", "R2-2601401"),
      });
    });

    await waitFor(() => {
      expect(startedQueries).toEqual(["R2-2601401", "38.321"]);
    });
    expect(await screen.findByText("Done")).toBeInTheDocument();

    act(() => {
      progressHandler?.({
        payload: {
          jobId: "job-2",
          stage: "error",
          message: "No archive file matched 38.321.",
          progress: 100,
          searchedUrlCount: 1,
        },
      });
    });

    await waitFor(() => {
      expect(startedQueries).toEqual(["R2-2601401", "38.321", "S2-260001"]);
    });
    expect(await screen.findByText("Error")).toBeInTheDocument();

    act(() => {
      completeHandler?.({
        payload: lookupCompletePayload("job-3", "S2-260001"),
      });
    });

    expect(await screen.findAllByText("Done")).toHaveLength(2);
    expect(screen.getByText("No archive file matched 38.321.")).toBeInTheDocument();
  });

  it("cancels only the active batch row and continues pending rows", async () => {
    const user = userEvent.setup();
    const startedQueries: string[] = [];

    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot: "Preview only",
          manifestCount: 0,
          recordCount: 0,
          indexCount: 0,
          lastCheckedAt: null,
        });
      }
      if (command === "start_gpp_lookup_job") {
        const request = (args as { request: { query: string } }).request;
        startedQueries.push(request.query);
        return Promise.resolve({ jobId: `job-${startedQueries.length}` });
      }
      if (command === "cancel_gpp_lookup_job") {
        return Promise.resolve(true);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);

    await user.type(await screen.findByRole("textbox", { name: "Batch queries" }), "R2-2601401\n38.321");
    await user.click(screen.getByRole("button", { name: "Start batch" }));
    await screen.findByRole("dialog", { name: /3gpp lookup progress/i });

    await user.click(screen.getByRole("button", { name: /close/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("cancel_gpp_lookup_job", { jobId: "job-1" });
      expect(startedQueries).toEqual(["R2-2601401", "38.321"]);
    });
    expect(await screen.findByText("Cancelled")).toBeInTheDocument();
    expect(await screen.findByText("Running")).toBeInTheDocument();
  });

  it("shows multiple exact candidates and waits for the user to choose one", async () => {
    const user = userEvent.setup();
    let progressHandler: ((event: { payload: unknown }) => void) | undefined;
    let candidatesHandler: ((event: { payload: unknown }) => void) | undefined;
    let completeHandler: ((event: { payload: unknown }) => void) | undefined;
    const selectedCandidate = {
      tdoc: "R2-2601401",
      sourceUrl:
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
      workGroup: "RAN2",
      meeting: "TSGR2_133bis",
    };

    listenMock.mockImplementation((event: string, handler: (event: { payload: unknown }) => void) => {
      if (event === "gpp-job-progress") {
        progressHandler = handler;
      }
      if (event === "gpp-job-candidates") {
        candidatesHandler = handler;
      }
      if (event === "gpp-job-complete") {
        completeHandler = handler;
      }
      return Promise.resolve(() => undefined);
    });
    invokeMock.mockImplementation((command: string) => {
      if (command === "gpp_catalog_status") {
        return Promise.resolve({
          catalogRoot: "Preview only",
          manifestCount: 0,
          recordCount: 0,
          indexCount: 0,
          lastCheckedAt: null,
        });
      }
      if (command === "start_gpp_lookup_job") {
        return Promise.resolve({ jobId: "job-1" });
      }
      if (command === "continue_gpp_lookup_with_candidate") {
        return Promise.resolve();
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<GppPage />);
    await user.type(await screen.findByRole("textbox", { name: /query/i }), "R2-2601401");
    await user.click(await screen.findByRole("button", { name: /find, download & open/i }));
    await waitFor(() => {
      expect(progressHandler).toBeDefined();
      expect(candidatesHandler).toBeDefined();
      expect(invokeMock).toHaveBeenCalledWith("start_gpp_lookup_job", expect.anything());
    });

    await act(async () => {
      progressHandler?.({
        payload: {
          jobId: "job-1",
          stage: "candidate",
          message: "2 exact candidates found. Select one to download.",
          progress: 58,
          searchedUrlCount: 24,
        },
      });
      candidatesHandler?.({
        payload: {
          jobId: "job-1",
          query: "R2-2601401",
          candidates: [
            selectedCandidate,
            {
              tdoc: "R2-2601401",
              sourceUrl:
                "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_134/Docs/R2-2601401.zip",
              workGroup: "RAN2",
              meeting: "TSGR2_134",
            },
          ],
        },
      });
    });

    const dialogTitle = await screen.findByText("Select 3GPP Candidate");
    const dialog = dialogTitle.closest(".ant-modal") as HTMLElement;
    expect(dialog).not.toBeNull();
    expect(within(dialog).getByText("TSGR2_133bis")).toBeInTheDocument();
    expect(within(dialog).getByText("TSGR2_134")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Last Lookup" })).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith(
      "continue_gpp_lookup_with_candidate",
      expect.anything(),
    );

    await user.click(within(dialog).getAllByRole("button", { name: "Download" })[0]);

    expect(invokeMock).toHaveBeenCalledWith("continue_gpp_lookup_with_candidate", {
      jobId: "job-1",
      candidate: selectedCandidate,
    });

    act(() => {
      completeHandler?.({
        payload: lookupCompletePayload("job-1", "R2-2601401"),
      });
    });

    expect(await screen.findByRole("heading", { name: "Last Lookup" })).toBeInTheDocument();
  });
});

function lookupCompletePayload(jobId: string, query: string) {
  return {
    jobId,
    query,
    sourceUrl: `https://www.3gpp.org/ftp/${query}.zip`,
    zipPath: `C:\\SpectrumPilotWorkspace\\3gpp\\${query}.zip`,
    extractedPath: `C:\\SpectrumPilotWorkspace\\3gpp\\${query}`,
    openedPath: `C:\\SpectrumPilotWorkspace\\3gpp\\${query}\\${query}.docx`,
    cacheStatus: "downloaded",
    message: `Downloaded and extracted ${query}.`,
  };
}
