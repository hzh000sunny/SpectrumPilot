import json
import tempfile
import unittest
from pathlib import Path

from build_compact_seed import build_compact_seed


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def sample_record(tdoc: str, size: int) -> dict:
    file_name = f"{tdoc}.zip"
    canonical_url = (
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/"
        f"{file_name}"
    )
    return {
        "schemaVersion": 1,
        "recordType": "tdoc-file",
        "id": f"file:{tdoc}",
        "canonicalUrl": canonical_url,
        "parentDirectoryUrl": "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        "root": "tsg_ran",
        "workGroupPath": "WG2_RL2",
        "workGroupCode": "RAN2",
        "meetingId": "meeting:tsg_ran/WG2_RL2/TSGR2_133bis",
        "meetingSlug": "TSGR2_133bis",
        "containerRole": "docs",
        "fileName": file_name,
        "extension": "zip",
        "remoteModifiedRaw": "04-03-26 09:50AM",
        "sizeRaw": str(size),
        "sizeBytes": size,
        "tdoc": {
            "key": tdoc,
            "prefix": "R2",
            "numberText": tdoc.removeprefix("R2-"),
            "yearHint": 2026,
        },
        "classification": {
            "isPrimaryTdoc": True,
            "isZip": True,
            "isIgnoredArtifact": False,
        },
    }


class BuildCompactSeedTests(unittest.TestCase):
    def test_builds_compact_records_and_index_from_legacy_shards(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "legacy"
            target = root / "compact-seed"
            write_json(
                source / "records" / "tdoc" / "RAN2" / "TSGR2_133bis.json",
                {
                    "schemaVersion": 1,
                    "recordType": "tdoc-meeting-records",
                    "workGroupCode": "RAN2",
                    "meetingSlug": "TSGR2_133bis",
                    "docsUrl": "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
                    "checkedAt": "2026-07-04T00:00:00Z",
                    "files": [
                        sample_record("R2-2601402", 1242240),
                        sample_record("R2-2601401", 80437),
                    ],
                },
            )

            summary = build_compact_seed(
                source=source,
                target=target,
                seed_version="test-seed",
                generated_at="2026-07-05T00:00:00Z",
                scope="unit test seed",
                force=True,
            )

            self.assertEqual(summary["recordCount"], 2)
            self.assertEqual(summary["meetingCount"], 1)
            self.assertEqual(summary["indexItemCount"], 2)
            self.assertEqual(summary["recordShardCount"], 1)
            self.assertEqual(summary["indexShardCount"], 1)

            seed = json.loads((target / "seed.json").read_text(encoding="utf-8"))
            self.assertEqual(seed["recordType"], "3gpp-catalog-seed")
            self.assertEqual(seed["seedVersion"], "test-seed")
            self.assertEqual(seed["catalogFormat"], "compact-v1")
            self.assertEqual(seed["recordCount"], 2)

            records = json.loads(
                (target / "compact" / "records" / "RAN2.json").read_text(encoding="utf-8")
            )
            self.assertEqual(records["recordType"], "tdoc-compact-records")
            self.assertEqual(records["workGroupCode"], "RAN2")
            self.assertEqual(records["meetings"][0]["id"], 0)
            self.assertEqual(records["meetings"][0]["docsPath"], "tsg_ran/WG2_RL2/TSGR2_133bis/Docs")
            self.assertEqual(
                records["meetings"][0]["files"][0],
                ["R2-2601401.zip", 80437, "04-03-26 09:50AM", "R2-2601401"],
            )

            index = json.loads(
                (target / "compact" / "index" / "R2_26.json").read_text(encoding="utf-8")
            )
            self.assertEqual(index["recordType"], "tdoc-compact-index")
            self.assertEqual(index["prefix"], "R2")
            self.assertEqual(index["year"], 2026)
            self.assertEqual(index["items"]["R2-2601401"], ["RAN2", 0, 0])
            self.assertEqual(index["items"]["R2-2601402"], ["RAN2", 0, 1])

            compact_summary = json.loads(
                (target / "compact" / "summary.json").read_text(encoding="utf-8")
            )
            self.assertEqual(compact_summary["recordCount"], 2)
            self.assertEqual(compact_summary["indexItemCount"], 2)

            manifest = json.loads(
                (target / "download-manifest.json").read_text(encoding="utf-8")
            )
            self.assertEqual(manifest["recordType"], "3gpp-catalog-download-manifest")
            self.assertEqual(manifest["seedVersion"], "test-seed")
            manifest_paths = {item["path"] for item in manifest["files"]}
            self.assertIn("seed.json", manifest_paths)
            self.assertIn("compact/summary.json", manifest_paths)
            self.assertIn("compact/records/RAN2.json", manifest_paths)
            self.assertIn("compact/index/R2_26.json", manifest_paths)
            self.assertGreater(manifest["totalSizeBytes"], 0)


if __name__ == "__main__":
    unittest.main()
