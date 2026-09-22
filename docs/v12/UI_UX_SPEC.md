# v12 UI/UX Specification

## Visual Direction

The interface must be simple, premium, and professional.

### Theme

- Background: soft slate/gray
- Primary: deep navy/indigo
- Accent cards: pastel blue, pastel green, pastel purple, pastel amber, pastel rose
- Text: slate/navy
- Borders: light slate
- Shadows: subtle
- Corners: rounded

### Typography

- Primary font: Inter or system UI fallback
- Consistent 16px base size
- Clear heading hierarchy
- No cramped form layouts

## Navigation

```text
Dashboard
New Process
Jobs
Results
Reports
Connections
Settings
Help
About
```

## Login Page

Fields:

- Username
- Password
- Sign in

Purpose:

- Consolidate job history by operator.
- Link every job, report, export, and configuration change to a user.

## Connection Manager

The first screen after selecting New Process should not auto-connect.

User enters:

- User
- Password

Optional advanced fields:

- Host, default `127.0.0.1`
- Port, default `3306`

Buttons:

- Connect
- Load .env
- Save .env
- Save as connection profile

After connecting:

- Database dropdown appears.
- Table dropdown appears after selecting database.
- Column metadata is loaded only after selecting table.

## Field Mapping

Dynamic dropdowns per source:

- Unique ID
- First Name
- Middle Name
- Last Name
- Birthdate
- Middle Initial

Optional fields:

- Barangay Code
- City/Municipality Code
- Region/Province fields
- Additional business fields

The app should show mapping completeness, datatype, and recommended aliases.

## Extra Fields

Two modes:

1. Visual selector
2. Advanced JSON

Example:

```json
{
  "source_fields": {
    "uuid": "source_uuid",
    "region_code": "source_region_code"
  },
  "base_fields": {
    "uuid": "matched_uuid",
    "pcn": "matched_pcn"
  }
}
```

## Destination Result Table

Inputs:

- Destination database
- Result table name
- Check table
- Create table

Rules:

- If table exists, display warning.
- Never overwrite silently.
- Allow append, replace, or choose another table only by explicit user selection.

## Matching Levels UI

Default view shows L1-L7 only.

Each level card displays:

- Level code
- Exact or fuzzy badge
- Fields used
- Confidence level
- Performance impact
- Short explanation

Advanced levels L8-L17 are collapsed and disabled unless expanded by the user.

## Live Monitor

Live monitor cards:

- Overall progress
- Current stage
- Current level
- Current batch
- Rows processed
- Rows/sec
- Matched
- Ambiguous
- Unmatched
- Invalid
- Duplicates
- ETA
- Elapsed time
- CPU
- RAM
- GPU

## Live Results Table

Columns:

- Time
- Level
- Match Type
- Source ID
- Source Name
- Matched ID
- Matched Name
- Final Score
- FN Score
- MN Score
- LN Score
- Candidate Count
- Decision

Click row to open Candidate Inspector.

## Candidate Inspector

Sections:

- Original source record
- Normalized source record
- Matched base record
- Normalized matched record
- Difference view
- Score breakdown
- Algorithm explanation
- Decision evidence

## Integrated Help

Every module must include:

- What is this?
- How it works
- Recommended settings
- Examples
- Risk warning if applicable

Example: Middle Initial

> Middle Initial is used to narrow possible matches. If the same initial is found but the full middle names are different, v12 does not automatically accept the match. It routes the record to fuzzy middle-name evaluation when fuzzy levels are enabled.
