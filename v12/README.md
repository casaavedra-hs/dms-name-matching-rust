# DMS Name Matching Enterprise v12

Status: Enterprise rebuild started.

This branch is the start of the v12 production architecture. v11 remains the last legacy portable build. v12 will be developed as a clean enterprise platform with a Rust matching engine, modern desktop/web UI, operator login, job history, dynamic source mapping, live monitoring, result table creation, and integrated documentation.

## Goals

- No Rust/Cargo/Python/Node dependency on end-user workstations.
- One production installer or portable package.
- Premium but simple interface.
- Login and history consolidation.
- Dynamic database/table/field mapping.
- L1-L7 as the default matching scope.
- L8+ only when explicitly enabled.
- Exact matching first, fuzzy only on remaining unmatched records.
- Batch/checkpoint/resume support.
- Live match result monitoring.
- Complete auditability and reconciliation.

## v12 Modules

1. Authentication and audit
2. Connection manager
3. Data source wizard
4. Dynamic field mapping
5. Extra fields JSON/selector
6. Destination result table manager
7. Matching engine pipeline
8. Live monitor
9. Results and candidate inspector
10. Reports and history
11. Built-in help and terminology explanations
12. Installer and release pipeline

## Matching Rules Preserved

- Suffix/extension excluded from matching.
- Near DOB: Year ±5, Month ±3, Day ±3.
- Middle initial comes from mapped `middle_initial` column.
- Same middle initial but different populated middle name is not auto-accepted; it is routed to fuzzy middle-name evaluation where applicable.
- Fuzzy scoring is field-by-field: first name, middle name, last name, not full-name concatenation.
- If matched at a stronger level, records do not proceed to weaker levels.

## First Development Milestone

M1 delivers the enterprise shell:

- Project structure
- UI design system
- Login flow
- Connection profile model
- Wizard skeleton
- Job registry schema
- Matching engine specification
- CI workflow updates
