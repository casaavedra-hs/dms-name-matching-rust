# v12 Architecture

## Product

DMS Name Matching Enterprise v12 is a desktop-first enterprise data-matching platform.

## Runtime Shape

```text
User
  ↓
Desktop App / Local Browser UI
  ↓
Rust Application Host
  ↓
Rust Matching Engine
  ↓
MySQL 8+
```

The end-user package must contain a compiled executable. It must not install Rust, Cargo, Node, Python, or other developer dependencies on the user workstation.

## Major Components

### App Host

Responsibilities:

- Start application process.
- Open the UI.
- Serve embedded assets.
- Manage sessions.
- Route API requests to internal services.
- Own shutdown and job-control signals.

### UI Layer

Responsibilities:

- Login page.
- Dashboard.
- New job wizard.
- Connections.
- Field mapping.
- Matching levels.
- Live monitor.
- Results viewer.
- Reports.
- History.
- Settings.
- Help center.

### Engine Layer

Responsibilities:

- Normalize records.
- Prepare indexes.
- Deduplicate when selected.
- Execute exact matching levels first.
- Execute fuzzy matching only for residual unmatched rows.
- Write matched, ambiguous, unmatched, and invalid records.
- Maintain checkpoints.
- Publish live progress events.

### Database Layer

Responsibilities:

- Manage connection profiles.
- Load databases/tables/columns.
- Validate mappings.
- Create destination result tables.
- Write jobs, logs, metrics, and reports.

### Reporting Layer

Responsibilities:

- Reconcile totals.
- Export CSV/JSON/PDF/Excel outputs.
- Generate executive and technical reports.
- Persist report metadata.

## Processing Pipeline

```text
Login
  ↓
Create or Resume Job
  ↓
Connect to MySQL
  ↓
Select Source Tables
  ↓
Map Required Fields
  ↓
Select Extra Fields
  ↓
Configure Destination Table
  ↓
Validate Plan
  ↓
Normalize and Stage
  ↓
Deduplicate, if enabled
  ↓
Exact Match L1-L7/L8+
  ↓
Remove Accepted Matches
  ↓
Fuzzy Match on Residuals
  ↓
Bulk Write Results
  ↓
Reconcile
  ↓
Generate Reports
```

## Design Principles

1. The UI explains every technical term.
2. The engine is auditable and deterministic.
3. No silent overwrites.
4. No black-box match decisions.
5. Fast exact matching before expensive fuzzy matching.
6. Adaptive batching, not fixed batch size.
7. CPU-first reliability; GPU is optional acceleration.
8. Every job can be resumed from a safe checkpoint.
9. Every output table is tied to a job and operator.
10. The application is usable by non-developers.
