# v12 Matching Engine Specification

## Execution Model

The v12 engine uses a streaming pipeline with controlled concurrency.

```text
Reader Workers
  ↓
Normalizer Workers
  ↓
Blocking / Candidate Generation
  ↓
Exact Matching Workers
  ↓
Fuzzy Matching Workers
  ↓
Bulk Result Writer
  ↓
Metrics Publisher
```

## Batch Strategy

Default batch size is adaptive, not fixed.

Initial recommendations:

| Available RAM | Initial Batch |
|---:|---:|
| 16 GB | 25,000 |
| 32 GB | 50,000 |
| 64 GB | 150,000 |
| 128 GB | 300,000 |

The engine can shrink or grow batch size based on memory pressure and writer latency.

## Threading and Parallelism

- Reader thread pool streams source rows.
- Normalization uses CPU worker threads.
- Exact matching uses indexed database joins and/or in-memory hash blocks.
- Fuzzy matching uses Rayon work-stealing workers.
- Result writer uses a separate queue so matching workers are not blocked by database inserts.

## Multiprocessing

The first production version will be multi-threaded in one process. Multiprocessing will be added once checkpoint boundaries and output-table locking are proven stable.

Planned multiprocessing layout:

```text
Coordinator Process
  ├─ Worker Process 1: batch range A
  ├─ Worker Process 2: batch range B
  ├─ Worker Process 3: batch range C
  └─ Writer Process: result merge and checkpoint journal
```

## Matching Order

1. Run all selected exact/deterministic levels in precedence order.
2. Accept strongest valid match.
3. Mark accepted source rows as matched.
4. Remove matched rows from later levels.
5. Run fuzzy levels only on residual unmatched rows.

## Default Levels

Default focus is L1-L7 only.

| Level | Type | Rule |
|---|---|---|
| L1 | Exact | FN + MN + LN + DOB |
| L2 | Exact | FN + MI + LN + DOB |
| L3 | Exact | FN + LN + DOB when middle is missing or compatible |
| L4 | Exact/Near DOB | FN + MN + LN + Near DOB |
| L5 | Fuzzy | FN + MN + LN + Exact DOB |
| L6 | Fuzzy | FN + LN + Exact DOB, without bypassing known middle-name conflict |
| L7 | Fuzzy | FN + MN + LN + Near DOB |

L8-L17 are advanced and disabled unless explicitly selected.

## Middle Name Rules

- `middle_initial` is mapped from a real DB column where available.
- If middle initial matches but full populated middle names are different, the record is not automatically accepted.
- The record proceeds to fuzzy middle-name evaluation when the selected level supports fuzzy middle scoring.

## Fuzzy Scoring

Fuzzy scoring is not full-name concatenation.

It is field-based:

```text
FN score
MN score
LN score
  ↓
weighted score
  ↓
threshold + margin validation
```

## Live Result Preview

During processing, a bounded result stream is written to the live monitor:

- time
- batch
- source unique ID
- source name
- base/matched unique ID
- matched name
- level
- exact/fuzzy
- final score
- FN/MN/LN score
- candidate count
- ambiguity reason

## Result Writing

The writer must use bulk inserts or staged CSV/LOAD DATA mode where available.

Recommended writer modes:

1. Batched insert every 10,000-50,000 rows.
2. Staged CSV + LOAD DATA for very large runs.
3. Transaction checkpoint after each completed batch.

## Reconciliation

A job is complete only if:

```text
Input rows = matched + ambiguous + unmatched + invalid
```

If this does not reconcile, job status becomes `VALIDATION_FAILED`, not `COMPLETED`.
