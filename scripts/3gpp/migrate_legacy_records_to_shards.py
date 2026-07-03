#!/usr/bin/env python3
"""Convert legacy one-file-per-TDoc records into sharded 3GPP seed JSON."""

from __future__ import annotations

import argparse
import json
import shutil
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--target", required=True, type=Path)
    parser.add_argument(
        "--checked-at",
        default=datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    )
    args = parser.parse_args()

    records = read_legacy_records(args.source)
    meeting_groups = group_meetings(records)
    index_groups = group_indexes(records)

    records_root = args.target / "records" / "tdoc"
    indexes_root = args.target / "indexes" / "tdoc"
    reset_dir(records_root)
    reset_dir(indexes_root)

    meeting_count = write_meeting_shards(records_root, meeting_groups, args.checked_at)
    index_count = write_index_shards(indexes_root, index_groups)

    print(f"legacy records read: {len(records)}")
    print(f"meeting shards written: {meeting_count}")
    print(f"index shards written: {index_count}")
    return 0


def read_legacy_records(source: Path) -> list[dict]:
    if not source.exists():
        raise SystemExit(f"source does not exist: {source}")

    records: list[dict] = []
    for path in sorted(source.glob("*.json")):
        with path.open("r", encoding="utf-8") as handle:
            record = json.load(handle)
        if record.get("recordType") != "tdoc-file":
            continue
        if not record.get("classification", {}).get("isPrimaryTdoc"):
            continue
        if not record.get("tdoc"):
            continue
        if not record.get("workGroupCode") or not record.get("meetingSlug"):
            continue
        records.append(record)
    return records


def group_meetings(records: list[dict]) -> dict[tuple[str, str], list[dict]]:
    groups: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for record in records:
        groups[(record["workGroupCode"], record["meetingSlug"])].append(record)
    return groups


def group_indexes(records: list[dict]) -> dict[tuple[str, int], list[dict]]:
    groups: dict[tuple[str, int], list[dict]] = defaultdict(list)
    for record in records:
        tdoc = record["tdoc"]
        prefix = tdoc.get("prefix")
        year = tdoc.get("yearHint")
        if not prefix or not isinstance(year, int):
            continue
        groups[(prefix, year)].append(record)
    return groups


def reset_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def write_meeting_shards(
    records_root: Path,
    groups: dict[tuple[str, str], list[dict]],
    checked_at: str,
) -> int:
    for (work_group, meeting), records in sorted(groups.items()):
        records = sorted(records, key=lambda record: record["fileName"])
        docs_url = records[0]["parentDirectoryUrl"]
        shard = {
            "schemaVersion": 1,
            "recordType": "tdoc-meeting-records",
            "workGroupCode": work_group,
            "meetingSlug": meeting,
            "docsUrl": docs_url,
            "checkedAt": checked_at,
            "files": records,
        }
        write_json(records_root / safe_component(work_group) / f"{safe_component(meeting)}.json", shard)
    return len(groups)


def write_index_shards(indexes_root: Path, groups: dict[tuple[str, int], list[dict]]) -> int:
    for (prefix, year), records in sorted(groups.items()):
        items = {}
        for record in sorted(records, key=lambda value: value["tdoc"]["key"]):
            tdoc = record["tdoc"]["key"]
            work_group = record["workGroupCode"]
            meeting = record["meetingSlug"]
            items[tdoc] = {
                "tdoc": tdoc,
                "fileName": record["fileName"],
                "url": record["canonicalUrl"],
                "workGroupCode": work_group,
                "meetingSlug": meeting,
                "recordShard": f"records/tdoc/{safe_component(work_group)}/{safe_component(meeting)}.json",
                "remoteModifiedRaw": record.get("remoteModifiedRaw"),
                "sizeBytes": record.get("sizeBytes"),
            }
        shard = {
            "schemaVersion": 1,
            "recordType": "tdoc-lookup-index",
            "prefix": prefix,
            "year": year,
            "items": items,
        }
        write_json(indexes_root / safe_component(prefix) / f"{year % 100:02}.json", shard)
    return len(groups)


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, ensure_ascii=True, indent=2)
        handle.write("\n")


def safe_component(value: str) -> str:
    return value.replace(":", "-").replace("/", "_").replace("\\", "_")


if __name__ == "__main__":
    raise SystemExit(main())
