# ============================================================================
# File: /tests/test_verify_checksum.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Unit test for /utils/verify_checksum.py integrity checker.
# Validates SHA256 logic against known values and malformed cases.
# ============================================================================

import hashlib
import tempfile
from utils.verify_checksum import calculate_sha256, load_checksum_file


def test_known_sha256_match():
    content = b"TestMediaMancerChecksum"
    with tempfile.NamedTemporaryFile(delete=False) as temp_file:
        temp_file.write(content)
        temp_path = temp_file.name

    expected = hashlib.sha256(content).hexdigest()
    actual = calculate_sha256(temp_path)

    assert actual == expected, "SHA256 does not match expected hash"


def test_load_checksum_file(tmp_path):
    expected = "abc123hashvalue"
    fake_checksum = tmp_path / "test.sha256"
    fake_checksum.write_text(f"{expected}  fakefile.zip\n")

    result = load_checksum_file(fake_checksum)
    assert result == expected


def test_invalid_checksum_file(tmp_path):
    empty_checksum = tmp_path / "blank.sha256"
    empty_checksum.write_text("")
    result = load_checksum_file(empty_checksum)
    assert result is None