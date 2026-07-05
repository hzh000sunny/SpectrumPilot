import json
import tempfile
import unittest
from pathlib import Path

from build_compact_seed import build_compact_seed, meeting_sort_key


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def sample_record(
    tdoc: str,
    size: int,
    meeting: str = "TSGR2_133bis",
    year: int = 2026,
) -> dict:
    file_name = f"{tdoc}.zip"
    canonical_url = (
        f"https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{meeting}/Docs/{file_name}"
    )
    return {
        "schemaVersion": 1,
        "recordType": "tdoc-file",
        "id": f"file:{tdoc}",
        "canonicalUrl": canonical_url,
        "parentDirectoryUrl": f"https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{meeting}/Docs/",
        "root": "tsg_ran",
        "workGroupPath": "WG2_RL2",
        "workGroupCode": "RAN2",
        "meetingId": f"meeting:tsg_ran/WG2_RL2/{meeting}",
        "meetingSlug": meeting,
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
            "yearHint": year,
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

    def test_builds_compact_seed_from_legacy_records_and_meeting_shards(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "catalog"
            target = root / "compact-seed"
            legacy_record = sample_record("R2-2601403", 7000, meeting="TSGR2_134", year=2026)
            shard_record = sample_record("R2-2601401", 80437, meeting="TSGR2_133bis", year=2026)
            duplicate_record = sample_record("R2-2601403", 7000, meeting="TSGR2_134", year=2026)
            write_json(source / "records" / "file-url-sha256-legacy.json", legacy_record)
            write_json(
                source / "records" / "tdoc" / "RAN2" / "TSGR2_133bis.json",
                {
                    "schemaVersion": 1,
                    "recordType": "tdoc-meeting-records",
                    "workGroupCode": "RAN2",
                    "meetingSlug": "TSGR2_133bis",
                    "docsUrl": "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
                    "checkedAt": "2026-07-04T00:00:00Z",
                    "files": [shard_record, duplicate_record],
                },
            )

            summary = build_compact_seed(
                source=source,
                target=target,
                seed_version="mixed-source-seed",
                generated_at="2026-07-05T00:00:00Z",
                scope="mixed legacy and sharded records",
                force=True,
            )

            self.assertEqual(summary["recordCount"], 2)
            self.assertEqual(summary["meetingCount"], 2)
            self.assertEqual(summary["indexItemCount"], 2)

            records = json.loads(
                (target / "compact" / "records" / "RAN2.json").read_text(encoding="utf-8")
            )
            meetings = {meeting["meetingSlug"]: meeting for meeting in records["meetings"]}
            self.assertEqual(set(meetings), {"TSGR2_133bis", "TSGR2_134"})
            self.assertEqual(meetings["TSGR2_134"]["files"][0][3], "R2-2601403")

            index = json.loads(
                (target / "compact" / "index" / "R2_26.json").read_text(encoding="utf-8")
            )
            self.assertIn("R2-2601401", index["items"])
            self.assertIn("R2-2601403", index["items"])

    def test_meeting_sort_places_number_before_suffix_variants(self) -> None:
        meetings = [
            "TSGR3_125",
            "TSGR3_123-bis",
            "TSGR3_123",
            "TSGR3_125-bis",
            "TSGR3_124",
        ]

        self.assertEqual(
            sorted(meetings, key=meeting_sort_key),
            [
                "TSGR3_123",
                "TSGR3_123-bis",
                "TSGR3_124",
                "TSGR3_125",
                "TSGR3_125-bis",
            ],
        )

    def test_duplicate_tdoc_index_keeps_first_sorted_meeting_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "catalog"
            target = root / "compact-seed"
            first_record = sample_record("R2-2601401", 1000, meeting="TSGR2_133", year=2026)
            duplicate_record = sample_record("R2-2601401", 2000, meeting="TSGR2_133bis", year=2026)
            first_record["id"] = first_record["canonicalUrl"]
            duplicate_record["id"] = duplicate_record["canonicalUrl"]
            write_json(
                source / "records" / "tdoc" / "RAN2" / "TSGR2_133bis.json",
                {
                    "schemaVersion": 1,
                    "recordType": "tdoc-meeting-records",
                    "workGroupCode": "RAN2",
                    "meetingSlug": "TSGR2_133bis",
                    "docsUrl": "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
                    "checkedAt": "2026-07-04T00:00:00Z",
                    "files": [duplicate_record],
                },
            )
            write_json(
                source / "records" / "tdoc" / "RAN2" / "TSGR2_133.json",
                {
                    "schemaVersion": 1,
                    "recordType": "tdoc-meeting-records",
                    "workGroupCode": "RAN2",
                    "meetingSlug": "TSGR2_133",
                    "docsUrl": "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133/Docs/",
                    "checkedAt": "2026-07-04T00:00:00Z",
                    "files": [first_record],
                },
            )

            build_compact_seed(
                source=source,
                target=target,
                seed_version="duplicate-test-seed",
                generated_at="2026-07-05T00:00:00Z",
                scope="duplicate key unit test",
                force=True,
            )

            records = json.loads(
                (target / "compact" / "records" / "RAN2.json").read_text(encoding="utf-8")
            )
            self.assertEqual(records["meetings"][0]["meetingSlug"], "TSGR2_133")
            self.assertEqual(records["meetings"][1]["meetingSlug"], "TSGR2_133bis")

            index = json.loads(
                (target / "compact" / "index" / "R2_26.json").read_text(encoding="utf-8")
            )
            self.assertEqual(index["items"]["R2-2601401"], ["RAN2", 0, 0])


if __name__ == "__main__":
    unittest.main()
