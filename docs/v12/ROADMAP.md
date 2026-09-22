# v12 Development Roadmap

## Milestone 1: Enterprise Shell

Deliverables:

- v12 project structure
- Login screen
- Dashboard shell
- Navigation shell
- Connection manager UI
- Database/table dropdown proof-of-concept
- Field mapping dropdown proof-of-concept
- Basic job history schema

## Milestone 2: Processing Wizard

Deliverables:

- Mode selection
- Source selection
- Required field mapping
- Extra fields selector and JSON editor
- Destination result table validation
- Matching level selection
- Review plan page

## Milestone 3: Matching Engine Integration

Deliverables:

- Existing v11 Rust matching engine integrated into v12 shell
- Exact-first orchestration
- Residual fuzzy orchestration
- Adaptive batch configuration
- Result queue and writer
- Checkpoint journal

## Milestone 4: Live Monitoring

Deliverables:

- Overall progress
- Level progress
- Batch progress
- Rows/sec
- ETA
- CPU/RAM/GPU metrics
- Live result preview
- Candidate inspector

## Milestone 5: Reports and History

Deliverables:

- Job registry
- Operator history
- Executive summary
- Technical report
- Reconciliation report
- Ambiguity report
- Performance report
- CSV/JSON/PDF/Excel export plan

## Milestone 6: Installer and Release

Deliverables:

- Windows portable package
- Windows installer
- SHA256 checksums
- GitHub Actions release pipeline
- User manual
- Admin guide
- Troubleshooting guide

## Definition of Done

A milestone is complete only when:

- It compiles in GitHub Actions.
- It launches on Windows x64.
- It has basic documentation.
- It has no known blocker that prevents the application from starting.
- It does not require Rust/Cargo on the target workstation.
