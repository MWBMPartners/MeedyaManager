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

## Caption naming, authoring and subtitle appearance follow-up

Follow the shared contract's [caption labels and explicit native authoring](https://github.com/MeedyaSuite/MeedyaSuite-specs/blob/docs/cross-app-media-contract/standards/CROSS-APP-MEDIA-CONTRACT.md#caption-labels-and-explicit-native-authoring) and [subtitle appearance/bitmap exploration](https://github.com/MeedyaSuite/MeedyaSuite-specs/blob/docs/cross-app-media-contract/standards/CROSS-APP-MEDIA-CONTRACT.md#subtitle-appearance-track-wide-placement-and-animated-bitmap-conversion). Proposed requirements include native 608/708 track/service naming, explicit subtitle-to-native-caption creation, whole-track PGS/DVB-Sub positioning and animated bitmap reconstruction. They do not claim implemented support or a delivery milestone.

Retain bold/italic/underline, colour/alpha, positioning, regions, fonts, outline/shadow, layering and dynamic styling whenever representable. Preserve exact originals and report source/target profile limits before conversion. Bitmap appearance is not semantic styling: OCR/style/animation recovery needs confidence and correction. Do not silently flatten animated captions or infer forced/SDH semantics from a display title.

Carry the practical native-caption investigation into fixtures: video-carried discovery beyond stream lists; both 608 fields and all 708 services; field-2-only samples; explicit inert packaging padding when required; evidenced source-to-presentation offsets; video association and caption ordering separate from defaults; independent payload/event readback. A backend's copy option is not proof of preservation. Full native 708 writing and Apple playback remain unverified.

The metadata-only naming trial verified a c608 handler label and existing object-name field while preserving media bytes, other tracks and caption sample tables. It did not validate universal title precedence or 708/player display. Prefer proven native naming for the chosen MP4/MOV profile; resolve localized tnam, object names and handler labels separately. Embedded services without a portable native label require a disclosed application/custom override, not silent stream splitting. Stable IDs, language, role, standard and service numbers survive renames.

Offer caption track/service inspection and safe renaming through the same edit/handoff model; show embedded versus local/custom labels and portability. Preserve native IDs, languages, roles, service inventory, defaults and references. Record source/target caption generation and PGS/DVB placement/animation transformations as derivations, not tag-only updates or duplicate originals. Rich subtitle appearances/resources must survive metadata enrichment/relocation. Use payload and semantic checks for unchanged media after a title edit; whole-file hashes necessarily change. Display tested naming/rendering coverage independently of successful writes.

Acceptance backlog: Unicode/localized/empty caption labels, multiple services, naming precedence and all supported layouts; no-op/idempotence and failed-write rollback; source-style/profile mismatches; generated native captions with bandwidth/character/overlap limits; both fields/services and independent seek/render checks; whole-track PGS/DVB offsets including clipping/shared resources; palette-only animation, karaoke sweeps and manual correction. Use synthetic/licensed fixtures; supplied animation examples and output-player tests remain future work.

### Service inventory and display-mode UX

One c608 container track can expose CC1/CC2 in field 1 and CC3/CC4 in field 2; MediaInfo service entries are not necessarily extra physical tracks. A generic trial exposed two different languages under one track-level English label. Keep service-level language/name evidence separate from inherited track tags, never infer language or duplicates from channel number, and preserve both. Explain pop-on (prepare off-screen then reveal a block), paint-on (write into displayed text) and roll-up (append/scroll lines). “Type of the first event” describes only that observation; modes can change and startup punctuation need not represent later dialogue. Preserve state, preloading, clear/display timing, styles and seek restoration. 708 services/windows and carried 608 compatibility data are separate identities; native 608 inside digital carriage is not automatically native 708. Follow the [shared service/display-mode requirements](https://github.com/MeedyaSuite/MeedyaSuite-specs/blob/docs/cross-app-media-contract/standards/CROSS-APP-MEDIA-CONTRACT.md#caption-tracks-services-and-display-modes), including explanatory UI and mode-switch/multilingual fixtures.

### Multilingual caption selection

Track titles must summarize all evidenced languages; keep track labels and per-service language/role evidence separate. A multilingual native language value (where supported) does not select a service. Player chooses a physical track then a 608 channel or 708 service using manual choice or validated language/role mapping, with explicit unknown/default fallback. CC1/CC2 share field 1; CC3/CC4 share field 2, and none prescribes a language. 708 service numbers 1–63 are separate identifiers. Text services/XDS are not ordinary dialogue. Respect descriptor-specific meaning: ATSC A/65:2013 does not give its language field meaning for 608 digital_cc=0. Keep inspected/user-labelled language evidence and unknown/conflicting values; do not silently infer or invent a portable map. See [shared language-selection requirements](https://github.com/MeedyaSuite/MeedyaSuite-specs/blob/docs/cross-app-media-contract/standards/CROSS-APP-MEDIA-CONTRACT.md#language-selection-and-multilingual-track-naming). Include bilingual, descriptor-conflict, empty-service and unsupported-engine fixtures.

### Default native-language caption titles

Use `<Native language name 1> / <Native language name 2> / … [CC]` for new caption authoring or explicit rename/normalization, for example `English / Español [CC]`. Use accurate Unicode autonyms, not anglicized names/transliteration. List evidenced languages once in stable service/source order; retain region/script distinctions, unknown/conflicting states, original titles and user overrides. Untouched remuxes preserve existing names. Keep BCP 47/native codes and service mappings separate from display labels. Test accents, RTL/mixed scripts, encoding/length limits, deterministic order and idempotence. Follow the [shared naming convention](https://github.com/MeedyaSuite/MeedyaSuite-specs/blob/docs/cross-app-media-contract/standards/CROSS-APP-MEDIA-CONTRACT.md#default-caption-title-convention); it is a Meedya UX policy, not an upstream naming mandate.
