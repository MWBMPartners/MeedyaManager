# ============================================================================
# File: /cli/metadata_debugger.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Command-line tool to debug metadata for a single file.
# Outputs metadata to console and supports export to JSON.
# ============================================================================

from utils.env_loader import load_env_variables
load_env_variables()

import os
import argparse
import json
from core.metadata_extractor import extract_metadata
from core.classify_media import classify_media
from utils.config_loader import get_config


def save_metadata_to_json(filepath, metadata, output_dir=None):
    base = os.path.basename(filepath)
    name, _ = os.path.splitext(base)
    json_filename = f"{name}.metadata.json"
    out_path = os.path.join(output_dir or os.path.dirname(filepath), json_filename)

    try:
        with open(out_path, 'w', encoding='utf-8') as f:
            json.dump(metadata, f, indent=4)
        print(f"[✅] Exported metadata to: {out_path}")
    except Exception as e:
        print(f"[❌] Failed to export JSON: {e}")


def main():
    parser = argparse.ArgumentParser(description="MediaMancer Metadata Debugger")
    parser.add_argument("filepath", help="Path to media file to analyze")
    parser.add_argument("--json", action="store_true", help="Export metadata to JSON file")
    parser.add_argument("--out", type=str, help="Output folder for JSON export")
    parser.add_argument("--mkdir", action="store_true", help="Create output folder if missing")
    args = parser.parse_args()

    if not os.path.isfile(args.filepath):
        print("[❌] File not found or not a regular file:", args.filepath)
        return

    if args.out and not os.path.exists(args.out):
        if args.mkdir:
            try:
                os.makedirs(args.out)
                print(f"[📁] Created output folder: {args.out}")
            except Exception as e:
                print(f"[❌] Failed to create output folder: {e}")
                return
        else:
            print(f"[❌] Output folder does not exist: {args.out}")
            return

    metadata = extract_metadata(args.filepath)
    metadata.update(classify_media(metadata))

    print("\n[🔍] Parsed Metadata:")
    for k, v in metadata.items():
        print(f"{k}: {v}")

    if args.json:
        save_metadata_to_json(args.filepath, metadata, args.out)


if __name__ == "__main__":
    main()