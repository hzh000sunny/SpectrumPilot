#!/usr/bin/env python3
"""Release-readiness checks for SpectrumPilot v0.1 packaging assets."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
EXPECTED_CATALOG_MANIFEST_URL = (
    "https://raw.githubusercontent.com/hzh000sunny/SpectrumPilot/"
    "v0.1.0/data/3gpp/catalog_seed/download-manifest.json"
)

sys.path.insert(0, str(Path(__file__).resolve().parent))


class ReleaseReadinessTests(unittest.TestCase):
    def test_bootstrap_catalog_manifest_url_is_release_pinned(self) -> None:
        seed_path = (
            REPO_ROOT
            / "apps"
            / "desktop"
            / "src-tauri"
            / "resources"
            / "3gpp"
            / "catalog_seed"
            / "seed.json"
        )
        seed = json.loads(seed_path.read_text(encoding="utf-8"))

        self.assertEqual(seed["catalogDownloadManifestUrl"], EXPECTED_CATALOG_MANIFEST_URL)
        self.assertNotIn("/main/", seed["catalogDownloadManifestUrl"])
        self.assertIn("/v0.1.0/", seed["catalogDownloadManifestUrl"])

    def test_compact_catalog_seed_validates_offline(self) -> None:
        from verify_compact_seed import verify_compact_seed

        report = verify_compact_seed(REPO_ROOT / "data" / "3gpp" / "catalog_seed")

        self.assertEqual(report["seedVersion"], "compact-stage-seed-2026-07-05-recent-2024-2026")
        self.assertEqual(report["fileCount"], 77)
        self.assertGreater(report["recordCount"], 200_000)
        self.assertGreater(report["indexItemCount"], 200_000)
        self.assertEqual(report["errorCount"], 0)

    def test_tauri_windows_updater_release_config_is_declared(self) -> None:
        config_path = REPO_ROOT / "apps" / "desktop" / "src-tauri" / "tauri.conf.json"
        config = json.loads(config_path.read_text(encoding="utf-8"))

        self.assertEqual(config["bundle"]["targets"], ["nsis", "msi"])
        self.assertTrue(config["bundle"]["createUpdaterArtifacts"])
        self.assertEqual(
            config["bundle"]["windows"]["webviewInstallMode"]["type"],
            "downloadBootstrapper",
        )
        updater = config["plugins"]["updater"]
        self.assertEqual(
            updater["endpoints"],
            ["https://github.com/hzh000sunny/SpectrumPilot/releases/latest/download/latest.json"],
        )
        self.assertEqual(updater["windows"]["installMode"], "passive")
        self.assertGreater(len(updater["pubkey"]), 20)
        self.assertNotIn("PRIVATE", updater["pubkey"].upper())


if __name__ == "__main__":
    unittest.main()
