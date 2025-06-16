# ============================================================================
# File: /cli/runner.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Command-line entry point to trigger background metadata scan and
# dry-run rename simulation. Supports optional JSON export for testing
# or scripting integration.
# ============================================================================

import os
import argparse
from core.metadata_extractor import extract_metadata
from core.classifier import classify_media
from utils.config_loader import get_config
from pprint import pprint
import json


def simulate_rename(filepath, metadata, dry_run=True, export_json=False, output_dir=None):
    print(f"\n[SIMULATION] Processing: {filepath}")
    pprint(metadata)

    if export_json:
        base = os.path.basename(filepath)
        name, _ = os.path.splitext(base)
        json_filename = f"{name}.metadata.json"

        if output_dir:
            out_path = os.path.join(output_dir, json_filename)
        else:
            out_path = os.path.join(os.path.dirname(filepath), json_filename)

        try:
            with open(out_path, 'w', encoding='utf-8') as f:
                json.dump(metadata, f, indent=4)
            print(f"[JSON] Exported metadata to: {out_path}")
        except Exception as e:
            print(f"[ERROR] Failed to export JSON: {e}")


def main():
    parser = argparse.ArgumentParser(description="MetaMancer Rename Simulator")
    parser.add_argument("--json", action="store_true", help="Export extracted metadata as JSON")
    parser.add_argument("--out", type=str, help="Optional output folder for JSON export")
    parser.add_argument("--mkdir", action="store_true", help="Create output folder if it does not exist")
    args = parser.parse_args()

    config = get_config()
    watch_folders = config.get("watch_folders", [])
    extensions = set(config.get("valid_extensions", []))

    if args.out:
        if not os.path.exists(args.out):
            if args.mkdir:
                try:
                    os.makedirs(args.out)
                    print(f"[INFO] Created output folder: {args.out}")
                except Exception as e:
                    print(f"[ERROR] Could not create output folder: {e}")
                    return
            else:
                print(f"[ERROR] Output folder does not exist: {args.out}")
                return

    print("[DRY RUN] Simulating renames for files in watch folders:\n")

    for folder in watch_folders:
        for root, _, files in os.walk(folder):
            for file in files:
                full_path = os.path.join(root, file)
                ext = os.path.splitext(file)[1].lower()
                if ext in extensions and os.path.isfile(full_path):
                    metadata = extract_metadata(full_path)
                    classified = classify_media(metadata)
                    metadata.update(classified)
                    simulate_rename(full_path, metadata, dry_run=True, export_json=args.json, output_dir=args.out)


if __name__ == "__main__":
    main()