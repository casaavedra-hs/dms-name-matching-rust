# v12 Built-in Help System

The application must explain terminology and algorithms inside each module.

## Help Format

Each help item uses this structure:

1. What is this?
2. Why it matters
3. Recommended setting
4. Accuracy impact
5. Performance impact
6. Example

## Required Help Topics

### Login

Explains why operator identity is required: history, audit trail, accountability, and reusable templates.

### Connection Profile

Explains MySQL credentials, environment files, saved profiles, and password storage.

### Database and Table

Explains source table, base table, and destination result table.

### Unique ID

Explains that Unique ID must identify a row/person in the selected source table and is used for result traceability.

### Middle Initial

Explains that middle initial narrows candidates but does not automatically override a full middle-name mismatch.

### Exact Match

Explains deterministic matching after normalization.

### Fuzzy Match

Explains field-by-field similarity scoring.

### Near DOB

Explains Year ±5, Month ±3, Day ±3.

### Deduplication

Explains within-table duplicate detection and self-match prevention.

### Extra Fields JSON

Explains how to include non-matching fields in the result table.

### Candidate Count

Explains that too many candidates may indicate a weak blocking rule and may require review.

### Ambiguity

Explains close-score candidates and why they must be reviewed rather than automatically accepted.

## Example Help Text: Fuzzy Match

Fuzzy matching is used when names are similar but not exactly equal. v12 compares first name, middle name, and last name separately. It does not join the full name into one string before scoring. This helps identify which part of the name caused the match or mismatch.

Example:

```text
Source:  Mercidita Peñalosa Alagon
Base:    Mercedita Penalosa Alagon

Normalized:
Source:  MERCIDITA PENALOSA ALAGON
Base:    MERCEDITA PENALOSA ALAGON

FN: similar
MN: exact after accent folding
LN: exact
Decision: fuzzy match if score and margin pass threshold
```

## Example Help Text: Destination Table

The destination table stores the final matching result. If the table already exists, v12 will not overwrite it automatically. You must choose whether to append, replace, or select another table name.
