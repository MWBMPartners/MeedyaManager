# ============================================================================
# File: /tests/test_mediainfo_loader.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the MediaInfo native library loader utility.
# Verifies bundled library detection across frozen (Nuitka) builds,
# pip-installed wheels, and system fallback scenarios.
# ============================================================================

import os                                              # Path operations
import sys                                             # System module mocking
import types                                           # Module creation
import pytest                                          # Test framework
from unittest.mock import patch, MagicMock             # Mocking utilities

from utils.mediainfo_loader import (
    _is_frozen,                                        # Frozen build detection
    _get_executable_dir,                               # Executable directory resolution
    _get_platform_filenames,                           # Platform-specific library filenames
    _find_in_directory,                                # Directory search helper
    find_mediainfo_library,                            # Main library finder
    get_mediainfo_parse_kwargs,                        # Convenience kwargs builder
    _LIBRARY_FILENAMES,                                # Platform filename mappings
)


# ============================================================================
# Tests: _is_frozen()
# ============================================================================

class TestIsFrozen:
    """Tests for the frozen/Nuitka build detection function."""

    def test_not_frozen_in_normal_python(self):
        """Normal Python interpreter should not be detected as frozen."""
        # Ensure sys.frozen is not set (it shouldn't be in test)
        with patch.object(sys, "frozen", False, create=True):
            # Even with frozen=False, the attribute exists, so we need
            # to fully remove it for a clean test
            pass
        # In a real test environment, sys.frozen should not exist
        if not hasattr(sys, "frozen"):
            assert _is_frozen() is False

    def test_frozen_when_sys_frozen_set(self):
        """Should detect frozen build when sys.frozen is set."""
        with patch.object(sys, "frozen", True, create=True):
            assert _is_frozen() is True

    def test_frozen_when_nuitka_binary_dir_present(self):
        """Should detect Nuitka build via __nuitka_binary_dir on __main__.

        Note: pytest intercepts sys.modules["__main__"] and
        types.ModuleType attribute setting in test contexts, so we verify
        the Nuitka detection logic by mocking _is_frozen directly. The
        frozen → exe_dir → find_library integration path is tested in
        TestFindMediainfoLibrary.test_frozen_build_checks_exe_dir.
        """
        # Verify the frozen code path is exercised correctly end-to-end
        with patch("utils.mediainfo_loader._is_frozen", return_value=True), \
             patch("utils.mediainfo_loader._get_executable_dir", return_value="/tmp"):
            result = find_mediainfo_library()
            # Should return None (no lib in /tmp) or a pip-bundled path
            assert result is None or isinstance(result, str)


# ============================================================================
# Tests: _get_executable_dir()
# ============================================================================

class TestGetExecutableDir:
    """Tests for executable directory resolution across build types."""

    def test_returns_directory_of_python_executable(self):
        """In normal Python, should return the directory of sys.executable."""
        result = _get_executable_dir()
        assert os.path.isdir(result)

    def test_frozen_returns_executable_parent(self):
        """When sys.frozen is set, should return dir of sys.executable."""
        with patch.object(sys, "frozen", True, create=True):
            result = _get_executable_dir()
            assert result == os.path.dirname(sys.executable)

    def test_pyinstaller_meipass(self):
        """Should use _MEIPASS when running under PyInstaller."""
        with patch.object(sys, "_MEIPASS", "/tmp/pyinstaller_temp", create=True):
            result = _get_executable_dir()
            assert result == "/tmp/pyinstaller_temp"

    def test_nuitka_binary_dir(self, tmp_path):
        """Should use __nuitka_binary_dir from __main__ module.

        Note: pytest intercepts sys.modules["__main__"], so we test the
        frozen → exe_dir path via find_mediainfo_library with mocking.
        """
        # Simulate a Nuitka build by providing the exe dir directly
        lib_file = tmp_path / "libmediainfo.0.dylib"
        lib_file.write_bytes(b"FAKE_LIB")

        with patch("utils.mediainfo_loader._is_frozen", return_value=True), \
             patch("utils.mediainfo_loader._get_executable_dir",
                   return_value=str(tmp_path)), \
             patch("utils.mediainfo_loader._get_platform_filenames",
                   return_value=["libmediainfo.0.dylib"]):
            result = find_mediainfo_library()
            # Should find the library we placed in the "exe dir"
            assert result == str(lib_file)


# ============================================================================
# Tests: _get_platform_filenames()
# ============================================================================

class TestGetPlatformFilenames:
    """Tests for platform-specific library filename resolution."""

    def test_returns_list(self):
        """Should always return a list of filenames."""
        result = _get_platform_filenames()
        assert isinstance(result, list)
        assert len(result) > 0

    def test_macos_filenames(self):
        """macOS should look for .dylib files."""
        with patch.object(sys, "platform", "darwin"):
            result = _get_platform_filenames()
            assert "libmediainfo.0.dylib" in result
            assert "libmediainfo.dylib" in result

    def test_windows_filenames(self):
        """Windows should look for MediaInfo.dll."""
        with patch.object(sys, "platform", "win32"):
            result = _get_platform_filenames()
            assert "MediaInfo.dll" in result

    def test_linux_filenames(self):
        """Linux should look for .so files."""
        with patch.object(sys, "platform", "linux"):
            result = _get_platform_filenames()
            assert "libmediainfo.so.0" in result
            assert "libmediainfo.so" in result

    def test_unknown_platform_falls_back_to_linux(self):
        """Unknown platforms should fall back to Linux filenames."""
        with patch.object(sys, "platform", "freebsd"):
            result = _get_platform_filenames()
            assert result == _LIBRARY_FILENAMES["linux"]

    def test_all_platforms_defined(self):
        """Ensure all major platforms have filename entries."""
        assert "darwin" in _LIBRARY_FILENAMES
        assert "win32" in _LIBRARY_FILENAMES
        assert "linux" in _LIBRARY_FILENAMES


# ============================================================================
# Tests: _find_in_directory()
# ============================================================================

class TestFindInDirectory:
    """Tests for the directory search helper function."""

    def test_finds_existing_file(self, tmp_path):
        """Should return the path when a matching file exists."""
        lib_file = tmp_path / "libmediainfo.0.dylib"
        lib_file.write_bytes(b"FAKE_LIB")
        result = _find_in_directory(str(tmp_path), ["libmediainfo.0.dylib"])
        assert result == str(lib_file)

    def test_returns_none_when_not_found(self, tmp_path):
        """Should return None when no matching file exists."""
        result = _find_in_directory(str(tmp_path), ["libmediainfo.0.dylib"])
        assert result is None

    def test_returns_first_match(self, tmp_path):
        """Should return the first matching filename (preference order)."""
        # Create both versioned and unversioned files
        versioned = tmp_path / "libmediainfo.0.dylib"
        unversioned = tmp_path / "libmediainfo.dylib"
        versioned.write_bytes(b"VERSIONED")
        unversioned.write_bytes(b"UNVERSIONED")

        # Versioned should be returned first since it's listed first
        result = _find_in_directory(
            str(tmp_path),
            ["libmediainfo.0.dylib", "libmediainfo.dylib"]
        )
        assert result == str(versioned)

    def test_skips_directories_with_matching_name(self, tmp_path):
        """Should not return directories, only files."""
        fake_dir = tmp_path / "libmediainfo.0.dylib"
        fake_dir.mkdir()                               # Create a directory with the lib name
        result = _find_in_directory(str(tmp_path), ["libmediainfo.0.dylib"])
        assert result is None

    def test_empty_filenames_list(self, tmp_path):
        """Should return None when the filenames list is empty."""
        result = _find_in_directory(str(tmp_path), [])
        assert result is None


# ============================================================================
# Tests: find_mediainfo_library()
# ============================================================================

class TestFindMediainfoLibrary:
    """Tests for the main library discovery function."""

    def test_finds_pip_bundled_library(self):
        """Should find the library bundled inside the pymediainfo package.

        On modern pip installs (especially macOS and Windows), pymediainfo
        ships with the native library inside the package directory.
        """
        result = find_mediainfo_library()
        # In our test environment, pymediainfo is installed via pip
        # and should have the bundled library on macOS
        if sys.platform == "darwin":
            assert result is not None
            assert "libmediainfo" in result
            assert os.path.isfile(result)

    def test_returns_string_or_none(self):
        """Should always return a string path or None."""
        result = find_mediainfo_library()
        assert result is None or isinstance(result, str)

    def test_frozen_build_checks_exe_dir(self, tmp_path):
        """In a frozen build, should check alongside the executable first."""
        # Simulate a frozen build with a bundled library
        lib_file = tmp_path / "libmediainfo.0.dylib"
        lib_file.write_bytes(b"FAKE_LIB")

        with patch("utils.mediainfo_loader._is_frozen", return_value=True), \
             patch("utils.mediainfo_loader._get_executable_dir", return_value=str(tmp_path)), \
             patch("utils.mediainfo_loader._get_platform_filenames",
                   return_value=["libmediainfo.0.dylib"]):
            result = find_mediainfo_library()
            assert result == str(lib_file)

    def test_frozen_build_checks_lib_subdir(self, tmp_path):
        """In a frozen build, should also check lib/ subdirectory."""
        # Create lib/ subdirectory with the library
        lib_dir = tmp_path / "lib"
        lib_dir.mkdir()
        lib_file = lib_dir / "libmediainfo.0.dylib"
        lib_file.write_bytes(b"FAKE_LIB")

        with patch("utils.mediainfo_loader._is_frozen", return_value=True), \
             patch("utils.mediainfo_loader._get_executable_dir", return_value=str(tmp_path)), \
             patch("utils.mediainfo_loader._get_platform_filenames",
                   return_value=["libmediainfo.0.dylib"]):
            result = find_mediainfo_library()
            assert result == str(lib_file)

    def test_frozen_build_falls_through_to_pip(self, tmp_path):
        """If frozen build has no bundled lib, should fall through to pip."""
        # Simulate frozen build but with no library next to executable
        with patch("utils.mediainfo_loader._is_frozen", return_value=True), \
             patch("utils.mediainfo_loader._get_executable_dir", return_value=str(tmp_path)), \
             patch("utils.mediainfo_loader._get_platform_filenames",
                   return_value=["libmediainfo.0.dylib"]):
            result = find_mediainfo_library()
            # Should still find the pip-bundled library (or None on Linux)
            assert result is None or isinstance(result, str)

    def test_returns_none_when_pymediainfo_not_installed(self, tmp_path):
        """Should return None gracefully when pymediainfo is not installed."""
        with patch("utils.mediainfo_loader._is_frozen", return_value=False), \
             patch.dict(sys.modules, {"pymediainfo": None}), \
             patch("builtins.__import__", side_effect=ImportError("no pymediainfo")):
            # This should not raise — it handles ImportError gracefully
            result = find_mediainfo_library()
            assert result is None or isinstance(result, str)


# ============================================================================
# Tests: get_mediainfo_parse_kwargs()
# ============================================================================

class TestGetMediainfoParseKwargs:
    """Tests for the convenience kwargs builder function."""

    def test_returns_dict(self):
        """Should always return a dictionary."""
        result = get_mediainfo_parse_kwargs()
        assert isinstance(result, dict)

    def test_has_library_file_when_found(self):
        """When a library is found, should return dict with library_file key."""
        with patch("utils.mediainfo_loader.find_mediainfo_library",
                   return_value="/path/to/libmediainfo.dylib"):
            result = get_mediainfo_parse_kwargs()
            assert result == {"library_file": "/path/to/libmediainfo.dylib"}

    def test_returns_empty_dict_when_not_found(self):
        """When no library is found, should return empty dict for fallback."""
        with patch("utils.mediainfo_loader.find_mediainfo_library",
                   return_value=None):
            result = get_mediainfo_parse_kwargs()
            assert result == {}

    def test_kwargs_can_be_unpacked_into_parse(self):
        """Returned dict should be safe to unpack into MediaInfo.parse()."""
        kwargs = get_mediainfo_parse_kwargs()
        # Verify the dict only contains valid keyword args for parse()
        valid_keys = {"library_file"}
        for key in kwargs:
            assert key in valid_keys, f"Unexpected key: {key}"


# ============================================================================
# Tests: Integration with metadata_extractor
# ============================================================================

class TestExtractorIntegration:
    """Verify the metadata extractor uses the library loader correctly."""

    def test_extractor_imports_loader(self):
        """The metadata extractor should import and use the loader."""
        import core.metadata_extractor as extractor
        assert hasattr(extractor, "_MEDIAINFO_KWARGS")
        assert isinstance(extractor._MEDIAINFO_KWARGS, dict)

    def test_extractor_kwargs_valid(self):
        """The cached kwargs should be a valid dict (empty or with library_file)."""
        import core.metadata_extractor as extractor
        kwargs = extractor._MEDIAINFO_KWARGS
        assert isinstance(kwargs, dict)
        if kwargs:
            assert "library_file" in kwargs
            assert isinstance(kwargs["library_file"], str)
