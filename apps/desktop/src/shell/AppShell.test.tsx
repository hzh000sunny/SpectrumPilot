import { fireEvent, render, screen } from "@testing-library/react";
import { ConfigProvider, theme } from "antd";
import { describe, expect, it } from "vitest";

import App from "../App";

function renderApp() {
  window.location.hash = "";

  return render(
    <ConfigProvider
      theme={{
        algorithm: theme.defaultAlgorithm,
        token: {
          colorPrimary: "#2563eb",
          borderRadius: 8,
        },
      }}
    >
      <App />
    </ConfigProvider>,
  );
}

describe("AppShell", () => {
  it("routes through the sidebar with 3GPP Ftp as the default page", async () => {
    renderApp();

    expect(await screen.findByText("SpectrumPilot")).toBeInTheDocument();
    expect(await screen.findByRole("heading", { level: 4, name: "3GPP Ftp" })).toBeInTheDocument();
    expect(await screen.findByText("3GPP FTP document lookup and download")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /sync/i })).not.toBeInTheDocument();
    expect(window.location.hash).toBe("#/3gpp");

    fireEvent.click(screen.getByRole("menuitem", { name: /settings/i }));

    expect(
      await screen.findByRole("heading", { level: 4, name: "Settings" }),
    ).toBeInTheDocument();
    expect(window.location.hash).toBe("#/settings");
    expect(screen.getByRole("menuitem", { name: /settings/i })).toHaveClass(
      "ant-menu-item-selected",
    );
    expect(screen.getByRole("menuitem", { name: /3gpp ftp/i })).not.toHaveClass(
      "ant-menu-item-selected",
    );
  });
});
