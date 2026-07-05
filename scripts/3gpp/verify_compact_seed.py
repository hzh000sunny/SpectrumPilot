#!/usr/bin/env python3
"""Verify a compact 3GPP catalog seed without network access."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


def verify_compact_seed(seed_root: Path) -> dict[str, Any]:
    seed_root = seed_root.resolve()
    manifest_path = seed_root / "download-manifest.json"
    seed_path = seed_root / "seed.json"
    summary_path = seed_root / "compact" / "summary.json"
    errors: list[str] = []

    manifest = read_json(manifest_path, errors)
    seed = read_json(seed_path, errors)
    summary = read_json(summary_path, errors)

    if manifest.get("recordType") != "3gpp-catalog-download-manifest":
        errors.append("download-manifest.json has unexpected recordType")
    if seed.get("recordType") != "3gpp-catalog-seed":
        errors.append("seed.json has unexpected recordType")
    if summary.get("recordType") != "tdoc-compact-summary":
        errors.append("compact/summary.json has unexpected recordType")

    manifest_files = manifest.get("files")
    if not isinstance(manifest_files, list):
        errors.append("download-manifest.json files must be an array")
        manifest_files = []

    if manifest.get("fileCount") != len(manifest_files):
        errors.append(
            f"manifest fileCount {manifest.get('fileCount')} does not match listed files {len(manifest_files)}"
        )

    seen_paths: set[str] = set()
    total_size = 0
    for entry in manifest_files:
        if not isinstance(entry, dict):
            errors.append("manifest contains a non-object file entry")
            continue
        relative_path = entry.get("path")
        if not isinstance(relative_path, str) or not relative_path:
            errors.append("manifest file entry has an invalid path")
            continue
        if relative_path in seen_paths:
            errors.append(f"manifest contains duplicate path: {relative_path}")
        seen_paths.add(relative_path)
        if is_unsafe_relative_path(relative_path):
            errors.append(f"manifest contains unsafe relative path: {relative_path}")
            continue
        file_path = seed_root / relative_path
        if not file_path.exists():
            errors.append(f"manifest path does not exist: {relative_path}")
            continue
        body = file_path.read_bytes()
        actual_size = len(body)
        actual_sha = hashlib.sha256(body).hexdigest()
        total_size += actual_size
        if entry.get("sizeBytes") != actual_size:
            errors.append(
                f"{relative_path} sizeBytes {entry.get('sizeBytes')} does not match {actual_size}"
            )
        if entry.get("sha256") != actual_sha:
            errors.append(f"{relative_path} sha256 does not match file content")

    if manifest.get("totalSizeBytes") != total_size:
        errors.append(
            f"manifest totalSizeBytes {manifest.get('totalSizeBytes')} does not match {total_size}"
        )

    actual_json_paths = {
        path.relative_to(seed_root).as_posix()
        for path in seed_root.rglob("*.json")
        if path.name != "download-manifest.json"
    }
    if actual_json_paths != seen_paths:
        missing = sorted(actual_json_paths - seen_paths)
        extra = sorted(seen_paths - actual_json_paths)
        if missing:
            errors.append(f"manifest omits JSON files: {', '.join(missing[:10])}")
        if extra:
            errors.append(f"manifest lists non-catalog files: {', '.join(extra[:10])}")

    if seed.get("seedVersion") != manifest.get("seedVersion"):
        errors.append("seedVersion differs between seed.json and download-manifest.json")

    for key in ("recordCount", "meetingCount", "indexItemCount"):
        if seed.get(key) != summary.get(key):
            errors.append(f"{key} differs between seed.json and compact/summary.json")

    record_shard_count, meeting_count, record_count = verify_record_shards(seed_root, errors)
    index_shard_count, index_item_count = verify_index_shards(seed_root, errors)
    if summary.get("recordShardCount") != record_shard_count:
        errors.append("recordShardCount differs from compact/records shard count")
    if summary.get("indexShardCount") != index_shard_count:
        errors.append("indexShardCount differs from compact/index shard count")
    if summary.get("meetingCount") != meeting_count:
        errors.append("meetingCount differs from compact record shards")
    if summary.get("recordCount") != record_count:
        errors.append("recordCount differs from compact record shards")
    if summary.get("indexItemCount") != index_item_count:
        errors.append("indexItemCount differs from compact index shards")

    return {
        "seedRoot": str(seed_root),
        "seedVersion": manifest.get("seedVersion"),
        "fileCount": len(manifest_files),
        "totalSizeBytes": total_size,
        "recordShardCount": record_shard_count,
        "indexShardCount": index_shard_count,
        "meetingCount": meeting_count,
        "recordCount": record_count,
        "indexItemCount": index_item_count,
        "errorCount": len(errors),
        "errors": errors,
    }


def verify_record_shards(seed_root: Path, errors: list[str]) -> tuple[int, int, int]:
    records_root = seed_root / "compact" / "records"
    shard_count = 0
    meeting_count = 0
    record_count = 0
    if not records_root.exists():
        errors.append("missing compact/records directory")
        return 0, 0, 0

    for path in sorted(records_root.glob("*.json")):
        shard_count += 1
        shard = read_json(path, errors)
        if shard.get("recordType") != "tdoc-compact-records":
            errors.append(f"{relative(seed_root, path)} has unexpected recordType")
            continue
        meetings = shard.get("meetings")
        if not isinstance(meetings, list):
            errors.append(f"{relative(seed_root, path)} meetings must be an array")
            continue
        meeting_count += len(meetings)
        for index, meeting in enumerate(meetings):
            if not isinstance(meeting, dict):
                errors.append(f"{relative(seed_root, path)} meeting {index} is not an object")
                continue
            if meeting.get("id") != index:
                errors.append(f"{relative(seed_root, path)} meeting id {meeting.get('id')} != {index}")
            files = meeting.get("files")
            if not isinstance(files, list):
                errors.append(f"{relative(seed_root, path)} meeting {index} files must be an array")
                continue
            record_count += len(files)
            for file_index, file_entry in enumerate(files):
                if not (isinstance(file_entry, list) and len(file_entry) >= 4):
                    errors.append(
                        f"{relative(seed_root, path)} meeting {index} file {file_index} is invalid"
                    )
    return shard_count, meeting_count, record_count


def verify_index_shards(seed_root: Path, errors: list[str]) -> tuple[int, int]:
    index_root = seed_root / "compact" / "index"
    records_by_group = load_record_shards(seed_root, errors)
    shard_count = 0
    item_count = 0
    if not index_root.exists():
        errors.append("missing compact/index directory")
        return 0, 0

    for path in sorted(index_root.glob("*.json")):
        shard_count += 1
        shard = read_json(path, errors)
        if shard.get("recordType") != "tdoc-compact-index":
            errors.append(f"{relative(seed_root, path)} has unexpected recordType")
            continue
        items = shard.get("items")
        if not isinstance(items, dict):
            errors.append(f"{relative(seed_root, path)} items must be an object")
            continue
        item_count += len(items)
        for key, pointer in items.items():
            if not (isinstance(pointer, list) and len(pointer) == 3):
                errors.append(f"{relative(seed_root, path)} item {key} has invalid pointer")
                continue
            work_group_code, meeting_id, file_index = pointer
            meeting_files = records_by_group.get(str(work_group_code), {}).get(meeting_id)
            if meeting_files is None:
                errors.append(
                    f"{relative(seed_root, path)} item {key} points to missing meeting {pointer}"
                )
                continue
            if not isinstance(file_index, int) or not 0 <= file_index < len(meeting_files):
                errors.append(
                    f"{relative(seed_root, path)} item {key} points to missing file {pointer}"
                )
                continue
            file_entry = meeting_files[file_index]
            if len(file_entry) >= 4 and file_entry[3] != key:
                errors.append(
                    f"{relative(seed_root, path)} item {key} points to record key {file_entry[3]}"
                )
    return shard_count, item_count


def load_record_shards(seed_root: Path, errors: list[str]) -> dict[str, dict[Any, list[list[Any]]]]:
    records: dict[str, dict[Any, list[list[Any]]]] = {}
    for path in sorted((seed_root / "compact" / "records").glob("*.json")):
        shard = read_json(path, errors)
        work_group_code = str(shard.get("workGroupCode") or path.stem)
        records[work_group_code] = {}
        meetings = shard.get("meetings") if isinstance(shard.get("meetings"), list) else []
        for meeting in meetings:
            if not isinstance(meeting, dict):
                continue
            files = meeting.get("files")
            if isinstance(files, list):
                records[work_group_code][meeting.get("id")] = files
    return records


def read_json(path: Path, errors: list[str]) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        errors.append(f"missing {path}")
        return {}
    except json.JSONDecodeError as source:
        errors.append(f"failed to parse {path}: {source}")
        return {}
    if not isinstance(payload, dict):
        errors.append(f"{path} must contain a JSON object")
        return {}
    return payload


def is_unsafe_relative_path(value: str) -> bool:
    path = Path(value)
    return path.is_absolute() or any(part == ".." for part in path.parts)


def relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("seed_root", type=Path)
    parser.add_argument("--json", action="store_true", help="print the full JSON report")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    report = verify_compact_seed(args.seed_root)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(
            "Verified {fileCount} files, {recordCount} records, {meetingCount} meetings, "
            "{indexItemCount} index items, {errorCount} errors".format(**report)
        )
        for error in report["errors"]:
            print(f"ERROR: {error}")
    raise SystemExit(1 if report["errorCount"] else 0)


if __name__ == "__main__":
    main()
