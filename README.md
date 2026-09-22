# DMS Name Matching — Rust

CI-verified high-volume name matching and deduplication engine.

## Production release policy

Target workstations do **not** build Rust locally. GitHub Actions compiles the native binaries on real Windows, Linux, and macOS runners. Windows users receive a precompiled EXE.

### Matching rules preserved

- Levels 1–7 are the core/default rules; only enabled levels execute.
- All selected exact/deterministic levels run before fuzzy levels.
- A record accepted at an earlier level does not continue to weaker levels.
- Near DOB: **Year ±5 / Month ±3 / Day ±3**.
- Suffix/extension is not used for matching.
- L2 uses middle initial, but same MI with different populated full middle names is not auto-accepted; it proceeds to fuzzy evaluation.
- L3/L6 do not bypass a known full-middle-name conflict.
- L5/L7 score FN, MN and LN by field.
- Deduplication never self-matches.
- Modes: Dedup Only, Name Matching Only, Dedup + Name Matching, Dedup → Consolidate → Name Match.

## Builds

The `Cross-platform CI` workflow runs tests and release builds on:
- Windows x64
- Linux x64
- macOS

The Windows artifact contains a precompiled `DMS_Name_Matching.exe`; no Rust, Cargo, Python, Node, Streamlit, compiler, or internet connection is required on the target workstation after download.


CI verification branch: cross-platform smoke/build validation.
