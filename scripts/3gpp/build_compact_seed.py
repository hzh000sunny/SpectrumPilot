#!/usr/bin/env python3
"""Build a compact 3GPP catalog seed from existing sharded JSON records.

This script is intentionally offline-only. It reads already fetched
`records/tdoc/**.json` meeting shards and rewrites them into a compact release
format with shared meeting paths and small prefix/year lookup shards.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from collections import defaultdict
from pathlib import Path
from typing import Any


BASE_3GPP_FTP_URL = "https://www.3gpp.org/ftp/"


def build_compact_seed(
    source: Path,
    target: Path,
    seed_version: str,
    generated_at: str,
    scope: str,
    force: bool = False,
) -> dict[str, Any]:
    source = source.resolve()
    target = target.resolve()
    if not (source / "records" / "tdoc").exists():
        raise FileNotFoundError(f"missing source records directory: {source / 'records' / 'tdoc'}")
    if target.exists():
        if not force:
            raise FileExistsError(f"target already exists: {target}")
        shutil.rmtree(target)

    compact_root = target / "compact"
    records_root = compact_root / "records"
    index_root = compact_root / "index"
    records_root.mkdir(parents=True, exist_ok=True)
    index_root.mkdir(parents=True, exist_ok=True)

    workgroup_meetings: dict[str, list[dict[str, Any]]] = defaultdict(list)
    index_items: dict[tuple[str, int], dict[str, list[Any]]] = defaultdict(dict)
    record_count = 0
    latest_checked_at: str | None = None

    for shard_path in sorted((source / "records" / "tdoc").rglob("*.json")):
        shard = read_json(shard_path)
        if shard.get("recordType") != "tdoc-meeting-records":
            continue
        work_group_code = require_text(shard, "workGroupCode", shard_path)
        meeting_slug = require_text(shard, "meetingSlug", shard_path)
        docs_url = require_text(shard, "docsUrl", shard_path)
        checked_at = require_text(shard, "checkedAt", shard_path)
        latest_checked_at = max_timestamp(latest_checked_at, checked_at)
        docs_path = canonical_docs_path(docs_url)

        files: list[list[Any]] = []
        meeting_id = len(workgroup_meetings[work_group_code])
        for record in sorted(shard.get("files", []), key=lambda item: item.get("fileName", "")):
            if not is_primary_tdoc_record(record):
                continue
            tdoc = record["tdoc"]
            key = require_text(tdoc, "key", shard_path)
            prefix = require_text(tdoc, "prefix", shard_path)
            year = int(tdoc["yearHint"])
            file_index = len(files)
            files.append(
                [
                    require_text(record, "fileName", shard_path),
                    record.get("sizeBytes"),
                    record.get("remoteModifiedRaw"),
                    key,
                ]
            )
            index_items[(prefix, year)][key] = [work_group_code, meeting_id, file_index]
            record_count += 1

        workgroup_meetings[work_group_code].append(
            {
                "id": meeting_id,
                "meetingSlug": meeting_slug,
                "docsPath": docs_path,
                "checkedAt": checked_at,
                "files": files,
            }
        )

    record_shard_count = 0
    for work_group_code, meetings in sorted(workgroup_meetings.items()):
        write_json(
            records_root / f"{safe_component(work_group_code)}.json",
            {
                "schemaVersion": 1,
                "recordType": "tdoc-compact-records",
                "workGroupCode": work_group_code,
                "baseUrl": BASE_3GPP_FTP_URL,
                "meetings": meetings,
            },
        )
        record_shard_count += 1

    index_shard_count = 0
    for (prefix, year), items in sorted(index_items.items()):
        write_json(
            index_root / f"{safe_component(prefix)}_{year % 100:02}.json",
            {
                "schemaVersion": 1,
                "recordType": "tdoc-compact-index",
                "prefix": prefix,
                "year": year,
                "items": dict(sorted(items.items())),
            },
        )
        index_shard_count += 1

    meeting_count = sum(len(meetings) for meetings in workgroup_meetings.values())
    index_item_count = sum(len(items) for items in index_items.values())
    summary = {
        "schemaVersion": 1,
        "recordType": "tdoc-compact-summary",
        "catalogFormat": "compact-v1",
        "recordCount": record_count,
        "meetingCount": meeting_count,
        "recordShardCount": record_shard_count,
        "indexShardCount": index_shard_count,
        "indexItemCount": index_item_count,
        "latestCheckedAt": latest_checked_at,
    }
    write_json(compact_root / "summary.json", summary)
    write_json(
        target / "seed.json",
        {
            "recordType": "3gpp-catalog-seed",
            "seedVersion": seed_version,
            "seedGeneratedAt": generated_at,
            "seedScope": scope,
            "catalogFormat": "compact-v1",
            "recordCount": record_count,
            "meetingCount": meeting_count,
            "indexItemCount": index_item_count,
        },
    )
    write_download_manifest(target, seed_version)
    return summary


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=True, separators=(",", ":")), encoding="utf-8")


def write_download_manifest(target: Path, seed_version: str) -> None:
    files: list[dict[str, Any]] = []
    for path in sorted(target.rglob("*.json")):
        relative_path = path.relative_to(target).as_posix()
        if relative_path == "download-manifest.json":
            continue
        body = path.read_bytes()
        files.append(
            {
                "path": relative_path,
                "sizeBytes": len(body),
                "sha256": hashlib.sha256(body).hexdigest(),
            }
        )
    total_size = sum(file["sizeBytes"] for file in files)
    write_json(
        target / "download-manifest.json",
        {
            "recordType": "3gpp-catalog-download-manifest",
            "schemaVersion": 1,
            "seedVersion": seed_version,
            "fileCount": len(files),
            "totalSizeBytes": total_size,
            "files": files,
        },
    )


def require_text(container: dict[str, Any], key: str, source_path: Path) -> str:
    value = container.get(key)
    if not isinstance(value, str) or not value:
        raise ValueError(f"missing {key} in {source_path}")
    return value


def canonical_docs_path(docs_url: str) -> str:
    if docs_url.startswith(BASE_3GPP_FTP_URL):
        path = docs_url[len(BASE_3GPP_FTP_URL) :]
    else:
        marker = "/ftp/"
        index = docs_url.find(marker)
        path = docs_url[index + len(marker) :] if index >= 0 else docs_url
    return path.strip("/")


def is_primary_tdoc_record(record: dict[str, Any]) -> bool:
    classification = record.get("classification") or {}
    tdoc = record.get("tdoc")
    return (
        bool(classification.get("isPrimaryTdoc"))
        and bool(classification.get("isZip"))
        and isinstance(tdoc, dict)
        and isinstance(tdoc.get("key"), str)
        and isinstance(tdoc.get("prefix"), str)
        and isinstance(tdoc.get("yearHint"), int)
    )


def safe_component(value: str) -> str:
    return "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in value)


def max_timestamp(current: str | None, candidate: str | None) -> str | None:
    if not candidate:
        return current
    if not current or candidate > current:
        return candidate
    return current


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--target", required=True, type=Path)
    parser.add_argument("--seed-version", required=True)
    parser.add_argument("--generated-at", required=True)
    parser.add_argument("--scope", required=True)
    parser.add_argument("--force", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    summary = build_compact_seed(
        source=args.source,
        target=args.target,
        seed_version=args.seed_version,
        generated_at=args.generated_at,
        scope=args.scope,
        force=args.force,
    )
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
