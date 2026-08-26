# Benchmark baselines

Criterion baseline `base`, captured 2026-08-26.

| | |
|---|---|
| Commit | `HEAD` |
| Branch | `master` |
| Toolchain | rustc 1.98.0 (88d9e12ae 2026-08-18) |
| Host | Linux x86_64 container (shared/virtualised) |

## Results

Mean with 95% confidence interval.

| Benchmark | Mean | 95% CI |
|---|---:|---|
| `generate_poisson_disk_circular/100@r100` | 279.4 µs | 278.4 µs – 280.6 µs |
| `hash1_u32` | 1.904 ns | 1.844 ns – 1.972 ns |
| `pick_weighted_index/100` | 295.5 ns | 289.3 ns – 303.2 ns |
| `tile_hash01` | 3.376 ns | 3.269 ns – 3.495 ns |

## Reproducing

The bench target must be named. A bare `cargo bench` also runs the lib test
harness, which rejects criterion's flags with
`error: Unrecognized option: 'baseline'`.

```sh
# capture
cargo bench --bench sampling -- --save-baseline base

# compare against it
cargo bench --bench sampling -- --baseline base
```

## How much to trust these

Taken in a shared virtualised container. Treat them as an order-of-magnitude
record, not a regression gate.

Re-running `take_batch/3x1000_pending` on byte-identical code about an hour
later on the *same* host reported `+31%` with `p = 0.00`. Criterion's
significance test measures sampling noise within a run; it cannot see the
host's load drifting between runs. So a reported change of this size here is
not evidence of a real regression.

To draw a conclusion from a comparison, capture the baseline and the
comparison back to back in one sitting, and treat anything under roughly
1.5x as inconclusive. Comparing these absolute numbers against a different
machine is meaningless.
