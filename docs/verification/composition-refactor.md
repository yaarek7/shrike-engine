# Search composition refactor verification

- Date: 2026-09-28

## Same-build A/B evidence

The poisoned-capture fixture `4k3/8/5n2/3p4/8/8/8/3QK3 w - - 0 1` is searched at depth one by two
`AlphaBetaSearcher` instances in the same test process:

- production configuration rejects `d1d5` and visits quiescence nodes;
- quiescence-disabled configuration selects `d1d5` and visits zero quiescence nodes.

A second comparison selects tactical versus encoded-only ordering and requires identical exact
scores. UCI composition is verified by injecting a backend owning a constant evaluator; its
distinct score appears in protocol output while the returned move remains legal.

The public integration suite also implements a minimal backend outside the search module and
constructs fixed and iterative results through exported APIs. This guards the extension seam from
becoming nominally public but unusable by a future methodology.

## Compatibility and performance boundary

Existing free functions retain the production defaults. `SearchBackend` dispatch occurs once per
root request. Recursive negamax and quiescence continue to use an evaluator reference and enum
policies, so the refactor adds no per-node virtual dispatch.

## Quality gate

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo test --release --test perft --locked -- --nocapture
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --release --locked
git diff --check
```
