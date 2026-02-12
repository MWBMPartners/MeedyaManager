# ============================================================================
# File: /utils/verify_checksum.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Post-install utility for end-users to verify integrity of downloaded
# ZIP or TAR.GZ archives against official SHA256 checksum files.
# This ensures tamper-free installation.
# ============================================================================

import hashlib
import argparse
from pathlib import Path


def calculate_sha256(file_path):
    """
    Reads file in chunks and returns SHA256 hash.
    """
    sha256 = hashlib.sha256()
    with open(file_path, "rb") as f:
        for block in iter(lambda: f.read(65536), b""):
            sha256.update(block)
    return sha256.hexdigest()


def load_checksum_file(checksum_path):
    """
    Extracts hash from checksum file of format:
    abcdef...  filename.ext
    """
    with open(checksum_path, "r") as f:
        line = f.readline().strip()
        return line.split()[0] if line else None


def main():
    parser = argparse.ArgumentParser(description="MediaMancer Checksum Verifier")
    parser.add_argument("file", help="Path to downloaded ZIP or TAR.GZ file")
    parser.add_argument("checksum", help="Path to accompanying .sha256 file")
    args = parser.parse_args()

    file_path = Path(args.file).resolve()
    checksum_path = Path(args.checksum).resolve()

    if not file_path.exists() or not checksum_path.exists():
        print("[❌] One or both files not found.")
        return

    expected_hash = load_checksum_file(checksum_path)
    actual_hash = calculate_sha256(file_path)

    print(f"\n🧾 Expected: {expected_hash}\n📦   Actual: {actual_hash}")

    if actual_hash.lower() == expected_hash.lower():
        print("✅ Checksum verified. File is intact.")
    else:
        print("❌ Checksum mismatch. File may be corrupted or tampered.")


if __name__ == "__main__":
    main()