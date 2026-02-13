# ============================================================================
# File: /tests/test_config_profile.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the configuration export/import profile module.
# Verifies path tokenization, validation, export/import round-trip,
# deep merge logic, change computation, and env template generation.
# ============================================================================

import os                                                  # Environment and path operations
import sys                                                 # Platform detection
import json                                                # JSON parsing for ZIP contents
import zipfile                                             # ZIP archive inspection
import pytest                                              # Test framework
from pathlib import Path                                   # Path operations
from unittest.mock import patch, MagicMock                 # Mocking utilities

from utils.config_profile import (
    get_platform_tokens,                                   # Platform token resolution
    normalize_paths_for_export,                            # Path → token conversion
    expand_paths_for_import,                               # Token → path conversion
    validate_profile,                                      # Profile validation
    export_profile,                                        # Profile export
    import_profile,                                        # Profile import
    preview_import,                                        # Import preview (dry-run)
    _deep_merge,                                           # Recursive dict merge
    _compute_changes,                                      # Config diff computation
    _build_env_template,                                   # Env key template builder
    _build_env_secrets,                                    # Env secrets builder
)


# ============================================================================
# Tests: get_platform_tokens()
# ============================================================================

class TestGetPlatformTokens:
    """Tests for the platform token resolution function."""

    def test_returns_dict(self):
        """Should return a dictionary of tokens."""
        tokens = get_platform_tokens()
        assert isinstance(tokens, dict)

    def test_contains_home_token(self):
        """Should include the {HOME} token."""
        tokens = get_platform_tokens()
        assert "{HOME}" in tokens
        assert tokens["{HOME}"] == str(Path.home())

    def test_contains_common_tokens(self):
        """Should include all standard platform tokens."""
        tokens = get_platform_tokens()
        required = ["{HOME}", "{DESKTOP}", "{DOWNLOADS}", "{MUSIC}", "{VIDEOS}"]
        for token in required:
            assert token in tokens, f"Missing token: {token}"

    def test_tokens_are_absolute_paths(self):
        """All token values should be absolute paths."""
        tokens = get_platform_tokens()
        for token, path in tokens.items():
            assert os.path.isabs(path), f"{token} is not absolute: {path}"

    def test_videos_token_varies_by_platform(self):
        """On macOS, VIDEOS should point to ~/Movies; elsewhere ~/Videos."""
        tokens = get_platform_tokens()
        home = str(Path.home())
        if sys.platform == "darwin":
            assert tokens["{VIDEOS}"] == os.path.join(home, "Movies")
        else:
            assert tokens["{VIDEOS}"] == os.path.join(home, "Videos")


# ============================================================================
# Tests: normalize_paths_for_export() / expand_paths_for_import()
# ============================================================================

class TestPathNormalization:
    """Tests for the path tokenization round-trip."""

    def test_normalizes_home_in_string(self):
        """Should replace the home directory with {HOME}."""
        home = str(Path.home())
        config = {"watch_paths": [f"{home}/Media"]}
        normalized = normalize_paths_for_export(config)
        assert normalized["watch_paths"][0] == "{HOME}/Media"

    def test_normalizes_nested_dicts(self):
        """Should handle nested dict values."""
        home = str(Path.home())
        config = {"settings": {"log_dir": f"{home}/logs"}}
        normalized = normalize_paths_for_export(config)
        assert normalized["settings"]["log_dir"] == "{HOME}/logs"

    def test_leaves_non_path_strings_unchanged(self):
        """Non-path strings should not be modified."""
        config = {"name": "my config", "count": 42}
        normalized = normalize_paths_for_export(config)
        assert normalized["name"] == "my config"
        assert normalized["count"] == 42

    def test_expand_reverses_normalize(self):
        """expand_paths_for_import should reverse normalize_paths_for_export."""
        home = str(Path.home())
        config = {"watch_paths": [f"{home}/Music/Library"]}
        normalized = normalize_paths_for_export(config)
        expanded = expand_paths_for_import(normalized)
        assert expanded["watch_paths"][0] == f"{home}/Music/Library"

    def test_expand_handles_all_tokens(self):
        """Should expand all known tokens to platform-specific paths."""
        tokens = get_platform_tokens()
        config = {token: f"{token}/subdir" for token in tokens}
        expanded = expand_paths_for_import(config)
        for token, path in tokens.items():
            assert expanded[token] == f"{path}/subdir"

    def test_does_not_modify_input(self):
        """normalize and expand should not mutate the input dict."""
        home = str(Path.home())
        config = {"path": f"{home}/test"}
        original_value = config["path"]
        normalize_paths_for_export(config)
        assert config["path"] == original_value

    def test_handles_lists_of_paths(self):
        """Should tokenize all entries in a list."""
        home = str(Path.home())
        config = {"paths": [f"{home}/a", f"{home}/b", "relative/c"]}
        normalized = normalize_paths_for_export(config)
        assert normalized["paths"][0] == "{HOME}/a"
        assert normalized["paths"][1] == "{HOME}/b"
        assert normalized["paths"][2] == "relative/c"


# ============================================================================
# Tests: validate_profile()
# ============================================================================

class TestValidateProfile:
    """Tests for the profile validation function."""

    def test_valid_profile(self, tmp_path):
        """Should return empty list for a valid profile."""
        profile = tmp_path / "valid.mmprofile"
        with zipfile.ZipFile(str(profile), "w") as zf:
            zf.writestr("manifest.json", json.dumps({"schema_version": 1}))
            zf.writestr("settings.json5", json.dumps({"test": True}))
        errors = validate_profile(profile)
        assert errors == []

    def test_missing_file(self, tmp_path):
        """Should report error for non-existent file."""
        errors = validate_profile(tmp_path / "nonexistent.mmprofile")
        assert len(errors) == 1
        assert "does not exist" in errors[0]

    def test_not_a_zip(self, tmp_path):
        """Should report error for non-ZIP file."""
        bad_file = tmp_path / "notzip.mmprofile"
        bad_file.write_text("this is not a zip file")
        errors = validate_profile(bad_file)
        assert len(errors) == 1
        assert "ZIP" in errors[0]

    def test_missing_manifest(self, tmp_path):
        """Should report error when manifest.json is missing."""
        profile = tmp_path / "no_manifest.mmprofile"
        with zipfile.ZipFile(str(profile), "w") as zf:
            zf.writestr("settings.json5", json.dumps({"test": True}))
        errors = validate_profile(profile)
        assert any("manifest" in e.lower() for e in errors)

    def test_missing_settings(self, tmp_path):
        """Should report error when settings.json5 is missing."""
        profile = tmp_path / "no_settings.mmprofile"
        with zipfile.ZipFile(str(profile), "w") as zf:
            zf.writestr("manifest.json", json.dumps({"schema_version": 1}))
        errors = validate_profile(profile)
        assert any("settings" in e.lower() for e in errors)

    def test_invalid_manifest_json(self, tmp_path):
        """Should report error for malformed manifest JSON."""
        profile = tmp_path / "bad_json.mmprofile"
        with zipfile.ZipFile(str(profile), "w") as zf:
            zf.writestr("manifest.json", "not json {{")
            zf.writestr("settings.json5", json.dumps({"test": True}))
        errors = validate_profile(profile)
        assert any("json" in e.lower() for e in errors)

    def test_missing_schema_version(self, tmp_path):
        """Should report error when manifest has no schema_version."""
        profile = tmp_path / "no_version.mmprofile"
        with zipfile.ZipFile(str(profile), "w") as zf:
            zf.writestr("manifest.json", json.dumps({"profile_name": "test"}))
            zf.writestr("settings.json5", json.dumps({"test": True}))
        errors = validate_profile(profile)
        assert any("schema_version" in e for e in errors)


# ============================================================================
# Tests: export_profile()
# ============================================================================

class TestExportProfile:
    """Tests for the profile export function."""

    def test_creates_zip_file(self, tmp_path):
        """Should create a valid ZIP file at the output path."""
        output = tmp_path / "test.mmprofile"
        with patch("utils.config_loader.get_config_path",
                   return_value=self._create_mock_config(tmp_path)):
            result = export_profile(output)
        assert Path(result).exists()
        assert zipfile.is_zipfile(result)

    def test_zip_contains_required_files(self, tmp_path):
        """ZIP should contain manifest.json, settings.json5, env.template."""
        output = tmp_path / "test.mmprofile"
        with patch("utils.config_loader.get_config_path",
                   return_value=self._create_mock_config(tmp_path)):
            result = export_profile(output)

        with zipfile.ZipFile(result, "r") as zf:
            names = zf.namelist()
            assert "manifest.json" in names
            assert "settings.json5" in names
            assert "env.template" in names

    def test_manifest_has_required_fields(self, tmp_path):
        """Manifest should have schema_version, profile_name, created_at."""
        output = tmp_path / "test.mmprofile"
        with patch("utils.config_loader.get_config_path",
                   return_value=self._create_mock_config(tmp_path)):
            result = export_profile(output, profile_name="Test Profile")

        with zipfile.ZipFile(result, "r") as zf:
            manifest = json.loads(zf.read("manifest.json"))
            assert manifest["schema_version"] == 1
            assert manifest["profile_name"] == "Test Profile"
            assert "created_at" in manifest

    def test_adds_mmprofile_extension(self, tmp_path):
        """Should add .mmprofile extension if missing."""
        output = tmp_path / "backup"
        with patch("utils.config_loader.get_config_path",
                   return_value=self._create_mock_config(tmp_path)):
            result = export_profile(output)
        assert result.endswith(".mmprofile")

    def test_no_secrets_by_default(self, tmp_path):
        """Should NOT include env.secrets unless explicitly opted in."""
        output = tmp_path / "test.mmprofile"
        with patch("utils.config_loader.get_config_path",
                   return_value=self._create_mock_config(tmp_path)):
            result = export_profile(output)

        with zipfile.ZipFile(result, "r") as zf:
            assert "env.secrets" not in zf.namelist()

    def test_includes_secrets_when_opted_in(self, tmp_path):
        """Should include env.secrets when include_secrets=True."""
        output = tmp_path / "test.mmprofile"
        with patch("utils.config_loader.get_config_path",
                   return_value=self._create_mock_config(tmp_path)):
            with patch.dict(os.environ, {"SPOTIFY_CLIENT_ID": "test_id"}):
                result = export_profile(output, include_secrets=True)

        with zipfile.ZipFile(result, "r") as zf:
            assert "env.secrets" in zf.namelist()
            secrets_text = zf.read("env.secrets").decode("utf-8")
            assert "SPOTIFY_CLIENT_ID=test_id" in secrets_text

    def test_paths_are_tokenized(self, tmp_path):
        """Exported settings should have paths replaced with tokens."""
        home = str(Path.home())
        config_path = self._create_mock_config(
            tmp_path, config={"watch_paths": [f"{home}/Music/Library"]}
        )
        output = tmp_path / "test.mmprofile"
        with patch("utils.config_loader.get_config_path",
                   return_value=config_path):
            result = export_profile(output)

        with zipfile.ZipFile(result, "r") as zf:
            settings = json.loads(zf.read("settings.json5"))
            # The home path should be tokenized
            assert any("{HOME}" in str(v) or "{MUSIC}" in str(v)
                       for v in str(settings).split(","))

    # --- Helper ---

    @staticmethod
    def _create_mock_config(tmp_path, config=None):
        """Create a temporary settings.json5 file and return its path."""
        import json5 as json5_lib
        config_data = config or {"watch_paths": ["./watch"], "valid_extensions": ["mp3"]}
        config_path = tmp_path / "settings.json5"
        with open(config_path, "w") as f:
            f.write(json5_lib.dumps(config_data))
        return str(config_path)


# ============================================================================
# Tests: import_profile()
# ============================================================================

class TestImportProfile:
    """Tests for the profile import function."""

    def _create_profile(self, tmp_path, config=None, profile_name="Test"):
        """Create a valid .mmprofile for testing."""
        config_data = config or {"watch_paths": ["./imported"], "valid_extensions": ["flac"]}
        profile_path = tmp_path / "test.mmprofile"
        manifest = {"schema_version": 1, "profile_name": profile_name}
        with zipfile.ZipFile(str(profile_path), "w") as zf:
            zf.writestr("manifest.json", json.dumps(manifest))
            zf.writestr("settings.json5", json.dumps(config_data))
        return profile_path

    def test_dry_run_returns_changes(self, tmp_path):
        """Dry run should return changes without applying."""
        profile = self._create_profile(tmp_path)
        current_config = {"watch_paths": ["./current"], "valid_extensions": ["mp3"]}
        mock_config_path = tmp_path / "current_settings.json5"
        mock_config_path.write_text(json.dumps(current_config))

        with patch("utils.config_loader.load_config", return_value=current_config):
            with patch("utils.config_loader.get_config_path",
                       return_value=str(mock_config_path)):
                result = import_profile(profile, dry_run=True)

        assert result["applied"] is False
        assert result["profile_name"] == "Test"
        assert len(result["changes"]) > 0

    def test_replace_mode_applies_full_config(self, tmp_path):
        """Replace mode should write the imported config to disk."""
        new_config = {"watch_paths": ["{HOME}/NewDir"], "valid_extensions": ["flac"]}
        profile = self._create_profile(tmp_path, config=new_config)
        current_config = {"watch_paths": ["./old"], "valid_extensions": ["mp3"]}
        config_file = tmp_path / "settings.json5"
        config_file.write_text(json.dumps(current_config))

        with patch("utils.config_loader.load_config", return_value=current_config):
            with patch("utils.config_loader.get_config_path",
                       return_value=str(config_file)):
                with patch("utils.config_loader.reload_config"):
                    result = import_profile(profile, mode="replace", dry_run=False)

        assert result["applied"] is True
        # Read the written config
        written = json.loads(config_file.read_text())
        # Paths should be expanded from tokens
        home = str(Path.home())
        assert written["watch_paths"][0] == f"{home}/NewDir"

    def test_merge_mode_preserves_existing_keys(self, tmp_path):
        """Merge mode should keep existing keys and add new ones."""
        new_config = {"new_key": "new_value", "valid_extensions": ["flac"]}
        profile = self._create_profile(tmp_path, config=new_config)
        current_config = {"existing_key": "preserved", "valid_extensions": ["mp3"]}
        config_file = tmp_path / "settings.json5"
        config_file.write_text(json.dumps(current_config))

        with patch("utils.config_loader.load_config", return_value=current_config):
            with patch("utils.config_loader.get_config_path",
                       return_value=str(config_file)):
                with patch("utils.config_loader.reload_config"):
                    result = import_profile(profile, mode="merge", dry_run=False)

        written = json.loads(config_file.read_text())
        assert written["existing_key"] == "preserved"
        assert written["new_key"] == "new_value"
        # Lists should be union-merged
        assert "mp3" in written["valid_extensions"]
        assert "flac" in written["valid_extensions"]

    def test_invalid_mode_raises(self, tmp_path):
        """Should raise ValueError for an unsupported mode."""
        profile = self._create_profile(tmp_path)
        with patch("utils.config_loader.load_config", return_value={}):
            with pytest.raises(ValueError, match="Unsupported import mode"):
                import_profile(profile, mode="invalid")

    def test_invalid_profile_raises(self, tmp_path):
        """Should raise ValueError for an invalid profile."""
        bad_file = tmp_path / "bad.mmprofile"
        bad_file.write_text("not a zip")
        with pytest.raises(ValueError, match="Invalid profile"):
            import_profile(bad_file)

    def test_preview_import_is_dry_run(self, tmp_path):
        """preview_import() should be equivalent to import_profile(dry_run=True)."""
        profile = self._create_profile(tmp_path)
        current_config = {"watch_paths": ["./current"]}

        with patch("utils.config_loader.load_config", return_value=current_config):
            with patch("utils.config_loader.get_config_path",
                       return_value=str(tmp_path / "settings.json5")):
                result = preview_import(profile)

        assert result["applied"] is False


# ============================================================================
# Tests: _deep_merge()
# ============================================================================

class TestDeepMerge:
    """Tests for the recursive dictionary merge function."""

    def test_overlay_adds_new_keys(self):
        """Overlay keys not in base should be added."""
        base = {"a": 1}
        overlay = {"b": 2}
        result = _deep_merge(base, overlay)
        assert result == {"a": 1, "b": 2}

    def test_overlay_replaces_scalars(self):
        """Overlay scalars should replace base scalars."""
        base = {"a": 1}
        overlay = {"a": 2}
        result = _deep_merge(base, overlay)
        assert result == {"a": 2}

    def test_nested_dicts_merged_recursively(self):
        """Nested dicts should be merged recursively."""
        base = {"nested": {"a": 1, "b": 2}}
        overlay = {"nested": {"b": 3, "c": 4}}
        result = _deep_merge(base, overlay)
        assert result == {"nested": {"a": 1, "b": 3, "c": 4}}

    def test_lists_merged_by_union(self):
        """Lists should be union-merged (no duplicates)."""
        base = {"ext": ["mp3", "flac"]}
        overlay = {"ext": ["flac", "wav"]}
        result = _deep_merge(base, overlay)
        assert set(result["ext"]) == {"mp3", "flac", "wav"}

    def test_does_not_mutate_inputs(self):
        """Merge should not modify either input dict."""
        base = {"a": {"x": 1}}
        overlay = {"a": {"y": 2}}
        base_copy = {"a": {"x": 1}}
        _deep_merge(base, overlay)
        assert base == base_copy


# ============================================================================
# Tests: _compute_changes()
# ============================================================================

class TestComputeChanges:
    """Tests for the configuration diff computation."""

    def test_detects_changed_value(self):
        """Should detect when a value changes."""
        old = {"key": "old_value"}
        new = {"key": "new_value"}
        changes = _compute_changes(old, new)
        assert "key" in changes
        assert changes["key"]["old"] == "old_value"
        assert changes["key"]["new"] == "new_value"

    def test_detects_added_key(self):
        """Should detect when a new key is added."""
        old = {}
        new = {"new_key": "value"}
        changes = _compute_changes(old, new)
        assert "new_key" in changes
        assert changes["new_key"]["old"] is None
        assert changes["new_key"]["new"] == "value"

    def test_detects_removed_key(self):
        """Should detect when a key is removed."""
        old = {"gone": "value"}
        new = {}
        changes = _compute_changes(old, new)
        assert "gone" in changes
        assert changes["gone"]["old"] == "value"
        assert changes["gone"]["new"] is None

    def test_no_changes_for_identical(self):
        """Should return empty dict when configs are identical."""
        config = {"a": 1, "b": "two", "c": [3]}
        changes = _compute_changes(config, config)
        assert changes == {}

    def test_nested_changes_use_dot_notation(self):
        """Nested dict changes should use dot-separated key paths."""
        old = {"parent": {"child": "old"}}
        new = {"parent": {"child": "new"}}
        changes = _compute_changes(old, new)
        assert "parent.child" in changes


# ============================================================================
# Tests: env template/secrets builders
# ============================================================================

class TestEnvBuilders:
    """Tests for the env.template and env.secrets builders."""

    def test_env_template_contains_key_names(self):
        """Template should list known API key names with blank values."""
        template = _build_env_template()
        assert "SPOTIFY_CLIENT_ID=" in template
        assert "TMDB_API_KEY=" in template

    def test_env_template_does_not_contain_values(self):
        """Template should not contain actual secret values."""
        template = _build_env_template()
        # Every key line should end with just "=" (blank value)
        for line in template.splitlines():
            if line and not line.startswith("#") and "=" in line:
                key, value = line.split("=", 1)
                assert value == "", f"Key {key} has a value: {value}"

    def test_env_secrets_includes_set_values(self):
        """Secrets file should include env vars that are set."""
        with patch.dict(os.environ, {"SPOTIFY_CLIENT_ID": "abc123"}):
            secrets = _build_env_secrets()
        assert "SPOTIFY_CLIENT_ID=abc123" in secrets

    def test_env_secrets_skips_unset_values(self):
        """Secrets file should skip env vars that are not set."""
        with patch.dict(os.environ, {}, clear=True):
            secrets = _build_env_secrets()
        # Only header lines, no key=value lines
        for line in secrets.splitlines():
            if line and not line.startswith("#"):
                assert False, f"Unexpected line in empty secrets: {line}"
