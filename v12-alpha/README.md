# DMS Name Matching Enterprise v12 Alpha

This folder contains the first working alpha shell for the v12 enterprise redesign.

## Current alpha scope

- Local Rust executable
- Premium pastel enterprise UI
- Login screen
- Dashboard
- Guided New Job workflow
- Dynamic-looking field mapping UI
- Extra Fields JSON editor
- Destination result table section
- Live monitor with running progress
- Running matched-results preview
- Specs used: threads, batch size, CPU/RAM/GPU display
- Help center with terminology and algorithm explanations

## Run locally

```bash
cd v12-alpha
cargo run --release
```

The app starts a local server on `127.0.0.1` and opens the browser automatically.

## Important alpha note

This alpha intentionally focuses on the application shell, UI/UX, live monitoring contract, and guided workflow. The next commits will connect the UI to real MySQL connection profiles, dynamic database/table/column discovery, result table creation, and the production matching engine.

## Next implementation targets

1. MySQL connection service
2. Database/table dropdowns from actual server
3. Dynamic column mapping from actual selected table
4. Destination table existence check and creation
5. Real job registry and audit tables
6. Real pipeline adapter to the v11 matching engine while v12 engine modules are built
