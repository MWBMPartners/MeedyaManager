# MeedyaManager: media preservation integration requirements

Status: proposed development/discovery requirements, 2026-10-11. No implemented support is claimed and existing milestones remain unchanged.

Use the [common media preservation and presentation contract](https://github.com/MeedyaSuite/MeedyaSuite-specs/blob/docs/cross-app-media-contract/standards/CROSS-APP-MEDIA-CONTRACT.md) alongside the [Converter research notes](https://github.com/MWBMPartners/MeedyaConverter/blob/docs/mp4-remux-preservation/docs/MP4-Remux-Preservation-Plan.md). These links reference draft working branches. Adopt a reviewed, commit-pinned contract before implementation/conformance claims; app-specific deviations need an explicit decision and capability report.

## Library and safe-editing requirements

Inventory all tracks/presentations, languages/roles, stereo/HDR data, captions, resources and relationships without reducing a multi-version file to one video/audio label. Preserve exact Kodi XML/NFO plus normalized metadata and provenance, unknown fields and repeated values. Embedded XML does not prove Kodi will consume it.

Organization/rename must preserve companion packages (NFO, artwork, subtitles/fonts and handoff documents), stable source identity and safe relationship locators. Relocation must not silently bind an external subtitle to the wrong edition. Avoid filename-only identity and bound filesystem lookup.

Metadata edits must leave unrelated encoded media, timing, codec configuration, native CEA-608/708 services, HDR/stereo and relationships unchanged. Whole-file hashes identify snapshots but cannot remain equal after an intentional tag edit; verify unchanged media/semantics separately from the expected metadata differences. Use transactional outputs, dependency-aware rewrites and rollback; refuse unsafe opaque-data relocation.

Duplicate recommendations may group equivalent versions; user-configurable quality preferences cannot silently delete a file/track or required fallback. Track-level evidence and edition/role/timing equivalence are distinct from library-file identity. Both caption standards stay protected unless explicitly excluded.

## Discovery and acceptance

Test tag-only updates on complex MP4/MOV/MKV fixtures, exact original XML retention, companion relocation, source reordering, duplicate recommendations, opaque data refusal and cancelled/full-disk edits. Verify payload/semantic invariants and typed relationship integrity, not just that the primary video plays.

## Planning integration

Carry these cases into future implementation issues and regression fixtures. Inventory/preview, safe edits and evidence contracts come first; edition-aware synchronization and incremental edits are bounded investigations. Do not infer support from a successful mux, codec label, progress message or an archived tag. Structural/sample checks, exhaustive preservation and target playback are different verification levels.
