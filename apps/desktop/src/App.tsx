import { App as AntApp } from "antd";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import "./App.css";
import { AppShell } from "./shell/AppShell";
import { DashboardPage } from "./pages/DashboardPage";
import { KeywordWatchlistPage } from "./pages/KeywordWatchlistPage";
import { ProposalLibraryPage } from "./pages/ProposalLibraryPage";
import { SettingsPage } from "./pages/SettingsPage";
import { GppPage } from "./pages/GppPage";

function App() {
  return (
    <AntApp>
      <HashRouter>
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<Navigate to="/3gpp" replace />} />
            <Route path="dashboard" element={<DashboardPage />} />
            <Route path="3gpp" element={<GppPage />} />
            <Route path="library" element={<ProposalLibraryPage />} />
            <Route path="watchlist" element={<KeywordWatchlistPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/3gpp" replace />} />
          </Route>
        </Routes>
      </HashRouter>
    </AntApp>
  );
}

export default App;
