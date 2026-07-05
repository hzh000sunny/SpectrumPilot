#!/usr/bin/env python3
"""Release-readiness checks for SpectrumPilot v0.1 packaging assets."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]

sys.path.insert(0, str(Path(__file__).resolve().parent))


class ReleaseReadinessTests(unittest.TestCase):
    def test_tauri_resources_bundle_full_compact_catalog_seed(self) -> None:
        from verify_compact_seed import verify_compact_seed

        resource_root = (
            REPO_ROOT
            / "apps"
            / "desktop"
            / "src-tauri"
            / "resources"
            / "3gpp"
            / "catalog_seed"
        )
        seed_path = resource_root / "seed.json"
        seed = json.loads(seed_path.read_text(encoding="utf-8"))

        self.assertEqual(seed["catalogFormat"], "compact-v1")
        self.assertNotIn("catalogDownloadManifestUrl", seed)
        self.assertTrue((resource_root / "compact" / "summary.json").exists())
        self.assertTrue((resource_root / "compact" / "records").exists())
        self.assertTrue((resource_root / "compact" / "index").exists())

        source_report = verify_compact_seed(REPO_ROOT / "data" / "3gpp" / "catalog_seed")
        resource_report = verify_compact_seed(resource_root)
        self.assertEqual(resource_report["errorCount"], 0)
        self.assertEqual(resource_report["seedVersion"], source_report["seedVersion"])
        self.assertEqual(resource_report["recordCount"], source_report["recordCount"])
        self.assertEqual(resource_report["indexItemCount"], source_report["indexItemCount"])
        self.assertEqual(
            count_json_files(resource_root),
            count_json_files(REPO_ROOT / "data" / "3gpp" / "catalog_seed"),
        )

    def test_desktop_does_not_start_default_catalog_download(self) -> None:
        lib_rs = (
            REPO_ROOT / "apps" / "desktop" / "src-tauri" / "src" / "lib.rs"
        ).read_text(encoding="utf-8")

        self.assertNotIn("spawn_background_gpp_catalog_seed_install", lib_rs)
        self.assertNotIn("install_configured_catalog_seed_for_app", lib_rs)
        self.assertNotIn("SPECTRUMPILOT_3GPP_CATALOG_MANIFEST_URL", lib_rs)

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


def count_json_files(root: Path) -> int:
    return sum(1 for _ in root.rglob("*.json"))


if __name__ == "__main__":
    unittest.main()
