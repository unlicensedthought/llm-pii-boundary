# Changelog

## 0.1.0 - 2026-11-18

- Rust library that replaces direct identifiers with placeholders before text goes to a model and restores them once in the answer.
- Deterministic detectors first (checksums for cards and IBANs), user-supplied terms, a legend for the model, and search-query handling.
- `pii-eval` plants known identifiers in 8 synthetic fixtures, searches everything that went out for them, and exits 1 on a single hit.
- `scripts/verify.sh` runs build, fmt, clippy, tests and the eval.
