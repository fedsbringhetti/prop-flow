<!--
Write the title and this body in English. Anything a stranger meets without opening
a file is English: commits, branch names, PR titles and bodies, tags, README.md,
CHANGELOG.md and release notes, job and step names in the workflows. Code, code
comments and docs/ stay in Italian.

Fill in every section. If one does not apply, say so explicitly instead of
deleting it — an absent section reads as an oversight, "not applicable" reads as a
decision.

This body is documentation. It must be enough for someone rereading it in a year
without opening the diff.
-->

## What this changes

<!-- The capability, in a paragraph. Not a list of files. -->

## Why

<!-- The problem this solves. If it fixes a defect, describe the defect and how it
was reproduced, not just the symptom. -->

## Design decisions

<!-- Each notable decision, with the alternative that was rejected and the reason.
Reference ADRs (docs/08-decisioni.md) for anything that outlives this PR. -->

## Schema and data

<!-- Migrations by number, RLS policies, constraints, indexes, privileges.
"None" if the PR does not touch the database. Remember: migrations are
append-only, and a SELECT policy must never filter deleted_at (ADR-008). -->

## API surface

<!-- New or changed endpoints, request/response shape, status codes, required role.
"None" if the PR does not touch the HTTP edge. -->

## Verification

<!-- What was run and what it produced. Numbers, not adjectives:

- `./ops/ci/test-integration.sh` — N tests, all passing
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — clean
- `cargo fmt --all --check` — clean
- `./ops/ci/check-domain-purity.sh` — 3/3
- schema invariants — N/N

Include anything verified by hand, and say how. -->

## Out of scope

<!-- What a reviewer might expect to find here and will not, with the reason and
where it lands instead. -->

## Review notes

<!-- Where to look first, the parts you are least sure about, anything that needs a
second opinion. Say so plainly — a PR that claims to be flawless invites a shallow
review. -->
