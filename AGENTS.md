# shr-lux working agreements

## Scope and evidence

Read README.md, idea.md and docs/index.md before substantial work. Preserve idea.md as
the original concept. Record new decisions and hardware findings in linked zk notes.
Label proposals, measured facts and unverified assumptions. Do not report USB detection
as proof that the DMX electrical output works.

## Engineering

Use Rust and the pinned toolchain. Target Raspberry Pi and an 80x25 terminal. Keep UI,
audio analysis, show decisions, fixture encoding and hardware I/O separate. Add modules
when needed rather than creating empty placeholder crates. Commit Cargo.lock.

Keep terminal-native mouse selection, context menus, clipboard and Ctrl+click links.
Do not enable mouse capture, intercept copy, or launch host browsers by default.
Restore terminal state on exit/error/panic. Make unavailable estimates explicit.

The scaffold must not send DMX. Future output requires explicit arming, a known fixture
patch, bounded I/O and defined blackout/recovery behavior. Do not guess fixture channel
maps, Arturia LED protocols, or hardware isolation. Never send bootloader requests as
part of diagnostics. Keep recordings and machine-specific credentials out of Git.

## Test selection and maintenance

The agent owns test classification based on changed behavior, risk and these instructions;
do not ask the user which tests to run. Keep fast production unit, contract, schema,
safety, recovery and focused regression checks in the normal suite.

Run the smallest focused tests while implementing. Run the complete normal production
suite for engine, render path, shared model/schema, routing/persistence, concurrency,
safety or broadly reused component changes, and before publication.

Historical research, audition generation, exhaustive matrices, long benchmarks and
one-time evidence renderers are opt-in or ignored after their evidence is recorded,
unless they protect current production behavior. Run them only when their assumptions,
code, evidence or directly protected behavior changes, or the user requests them.
If slow one-time tests enter the default suite, classify and mark them within the scoped
change and document the command to run them. Report classes run and intentionally skipped.

Normal checks: cargo fmt --all -- --check; cargo clippy --locked --all-targets -- -D warnings;
cargo test --locked --all-targets. Physical fixture tests are separate and never automatic
in CI. Update the notebook with significant validation evidence and remaining limitations.

Also run the fast Linux PTY suite after the release build: python3 scripts/check-terminal.py.

## GigPies task tracking

Use `docs/notes/0027-gigpies-implementation.md` for GigPies work owned here. Keep each task plan,
implementation state, acceptance checklist, evidence and next action in the same
card; update it with the change. Shared integration tasks have one card in
GigPies, linked from contributor plans. STATUS, maps, handoffs and knowledge notes
route to task owners or preserve dated evidence; never mirror current task state.
Archive closed cards once; keep the active queue limited to open work. Reference
projects do not become GigPies runtime modules merely because code is reused.

## Reader-facing documentation

Keep README short and visual. Route musicians through the operating guide;
assume musical knowledge. Put protocols, implementation detail, validation and
work logs in engineering references or archives. Update the existing owning
guide instead of appending feature histories to the README.
