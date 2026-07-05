import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProposalLibraryPage } from "./ProposalLibraryPage";

const invokeMock = vi.hoisted(() => vi.fn());
const isTauriMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

describe("ProposalLibraryPage", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReturnValue(true);
  });

  it("shows lookup history from the local 3GPP library", async () => {
    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "gpp_lookup_history") {
        expect(args).toEqual({ limit: 100 });
        return Promise.resolve([
          {
            schemaVersion: 1,
            recordType: "lookup-history",
            query: "R2-2601401",
            sourceUrl:
              "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
            zipPath:
              "C:\\SpectrumPilotWorkspace\\3gpp\\tdocs\\RAN2\\TSGR2_133bis\\R2-2601401\\R2-2601401.zip",
            extractedPath:
              "C:\\SpectrumPilotWorkspace\\3gpp\\tdocs\\RAN2\\TSGR2_133bis\\R2-2601401",
            openedPath:
              "C:\\SpectrumPilotWorkspace\\3gpp\\tdocs\\RAN2\\TSGR2_133bis\\R2-2601401\\R2-2601401.docx",
            cacheStatus: "downloaded",
            message: "Downloaded and extracted R2-2601401.",
            completedAt: "2026-07-04T08:00:00Z",
          },
        ]);
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<ProposalLibraryPage />);

    expect(await screen.findByRole("heading", { name: "Proposal Library" })).toBeInTheDocument();
    expect(await screen.findByText("R2-2601401")).toBeInTheDocument();
    expect(await screen.findByText("Downloaded from 3GPP FTP")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getAllByText(/TSGR2_133bis/).length).toBeGreaterThan(0);
    });
    expect(screen.queryByText(/will be searchable/i)).not.toBeInTheDocument();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("gpp_lookup_history", { limit: 100 });
    });
  });
});
