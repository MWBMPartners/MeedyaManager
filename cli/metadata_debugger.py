# ============================================================================
# File: /cli/metadata_debugger.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# CLI utility for developers and power users to debug metadata extraction.
# Prints detailed extracted and classified metadata for a given file.
# Supports optional export to .json for scripting and automation.
# ============================================================================

import sys
import os
import json
from core.metadata_extractor import extract_metadata
from pprint import pprint


def main():
    if len(sys.argv) < 2:
        print("Usage: python cli/metadata_debugger.py /path/to/media.file [--json] [--out /path/to/dir]")
        sys.exit(1)

    file_path = sys.argv[1]
    export_json = "--json" in sys.argv
    out_dir = None

    # Check for output directory override
    if "--out" in sys.argv:
        try:
            out_index = sys.argv.index("--out")
            out_dir = sys.argv[out_index + 1]
            if not os.path.isdir(out_dir):
                print(f"[ERROR] Output directory does not exist: {out_dir}")
                sys.exit(1)
        except IndexError:
            print("[ERROR] --out specified but no directory given")
            sys.exit(1)

    if not os.path.isfile(file_path):
        print(f"[ERROR] File does not exist: {file_path}")
        sys.exit(1)

    metadata = extract_metadata(file_path)

    if export_json:
        base_name = os.path.splitext(os.path.basename(file_path))[0]
        json_output_path = os.path.join(out_dir if out_dir else os.path.dirname(file_path), base_name + ".metadata.json")
        with open(json_output_path, "w", encoding="utf-8") as f:
            json.dump(metadata, f, indent=2, ensure_ascii=False)
        print(f"[OK] Metadata exported to: {json_output_path}")
    else:
        print("\n================ Metadata Debugger Output ================")
        pprint(metadata)
        print("=========================================================\n")


if __name__ == "__main__":
    main()