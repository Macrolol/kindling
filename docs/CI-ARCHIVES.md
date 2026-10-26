# CI run archives

The `headless-sims` workflow (`.github/workflows/headless-sims.yml`, manual dispatch) keeps its
outputs in two places:

| where | what | kept |
|---|---|---|
| Actions artifacts of the run (`results-<seeds>`, `summary`) | per-job reports, structured outputs, saves; combined summary | 30 days (the build binary: 1 day) |
| Pre-release **`run-<run id>`** (Releases page) | one archive of everything, input `archive` (default on) | until deleted |

## The archive

- Asset `kindling-run-<id>.tar.zst` (tar + zstd -19 --long=27), plus
  `kindling-run-<id>-manifest.json` (the same `run-manifest.json` as inside the archive) and
  `kindling-run-<id>.sha256`. Archives over 2 GB (the asset limit) are split into
  `kindling-run-<id>.tar.zst.part00`, `.part01`, ... .
- Contents:
  - `run-manifest.json`: run id and URL, the commit, every input, seeds, map, years, start and
    archive times, and every file with its size.
  - `jobs/results-<seeds>/`: the text report (`results-*.txt`), stderr, the structured outputs
    (`data/seed-S/`: manifest.json, metrics.csv, summary.json, chronicle.ndjson, frames.ndjson;
    see docs/DATA_SCHEMA.md), screenshots, and the `.civsave` files **only** with
    `archive_saves` on.
  - `summary/`: the combined report, aggregate tables, runs.csv and metrics-all.csv.
- The tag points at the commit that was simulated. The pre-release is never marked latest and
  is separate from the version releases (`v0.x`).

## Getting one

```
gh release list -R Macrolol/kindling                      # run-<id> entries are the CI runs
gh release download run-<id> -R Macrolol/kindling -D run-<id>
cd run-<id> && sha256sum -c kindling-run-<id>.sha256
cat kindling-run-<id>.tar.zst.part* > kindling-run-<id>.tar.zst   # only if split
tar --zstd -xf kindling-run-<id>.tar.zst                  # or: zstd -d --long=27 ... | tar -x
python3 tools/collect_runs.py kindling-run-<id>/jobs      # runs.csv + metrics-all.csv
```

`--long=27` needs `zstd -d --long=27` (or `--memory=128MB`) with older zstd versions; GNU tar's
`--zstd` handles it with zstd 1.5+.

## Public: never archive user saves

Releases of this repository are **public**. The archive only ever holds what the CI run itself
generated (fresh worlds from seeds). Never put a user's or a player's save into a workflow
input, the repository or an archive. `archive_saves` is for CI-generated saves only.

To remove a test archive: `gh release delete run-<id> -R Macrolol/kindling --cleanup-tag --yes`
(deletes that pre-release and its tag only).
