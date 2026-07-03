import { describe, expect, it, vi } from "vitest";

import { getRuntimeSnapshot } from "./runtime";

const invokeMock = vi.hoisted(() => vi.fn());
const isTauriMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

describe("getRuntimeSnapshot", () => {
  it("returns a browser preview snapshot outside the Tauri runtime", async () => {
    isTauriMock.mockReturnValue(false);

    const snapshot = await getRuntimeSnapshot();

    expect(invokeMock).not.toHaveBeenCalled();
    expect(snapshot.status).toBe("SpectrumPilot browser preview");
    expect(snapshot.paths.threeGppCatalogDir).toBe("Preview only");
  });
});
