import { act, render, screen } from "@testing-library/react";
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
});
