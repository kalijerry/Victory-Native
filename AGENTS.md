# AGENTS.md: working in Victory Native

For everyone who changes this tree, people and AI agents alike. Read [README.md](README.md)
and [CONTRIBUTING.md](CONTRIBUTING.md) first.

## The rule: translate, don't fake

It applies to every component: rendering, shaders, ray tracing, memory, audio,
synchronisation, streaming and system APIs.

1. Translate the behaviour of the Xbox hardware and APIs into its PC equivalent.
2. Where no native equivalent exists, reproduce the semantics, and test them.
3. Never invent a result to hide a feature that is wrong or missing: no stand-in light, no
   made-up value or success code, no empty or constant texture, no silently dropped work.
4. Never mistake a visual enhancement for a renderer fix.
5. Fix root causes. Workarounds are allowed only to diagnose or to unblock an investigation:
   optional, off by default, listed in [docs/workarounds.md](docs/workarounds.md), and logged
   when they act. Each one has a named switch and a way out.
6. Optimise without changing what the game is meant to do. A faster path must give the same
   result; prove it with a test, a comparison or an A/B run.
7. A problem is fixed when measurements show it. Until then, say "not fixed" or "not proven".
8. Never hide an error, a crash, an incompatibility or a regression. Log it, list it, report
   it.

If you meet inherited code that breaks the rule, do not extend it. List it in
[docs/shortcuts-audit.md](docs/shortcuts-audit.md) or [docs/workarounds.md](docs/workarounds.md)
and plan the real implementation.

## Measuring

- Build: `cargo build --locked --profile diag -p victory --features diagnostics`. Use a
  `CARGO_TARGET_DIR` of your own per worktree.
- A run is comparable only to a run made on the same machine, save, scripted input
  (`VICTORY_KEYS`), switches, profile and warm pipeline cache (`victory warm --used` before each
  run). Never set stand-in switches (`VICTORY_AMBIENT*`) in a measurement.
- Report the frame time over a fixed window and say which side limits it: card busy versus
  waiting for the CPU (the frame line's "the card's clock per frame").
- For the picture, compare contact sheets or clips of the same shots. Count flicker with an
  objective metric, not only by eye.
- No build while a measurement runs on the same machine: it skews CPU numbers. Only one game
  runs at a time; runs are queued.
- One run is noisy (±10%). Repeat before concluding, or use an in-run A/B (`VICTORY_AB`).

## Working together

- Each person or agent works in a git worktree and on a branch of its own.
- Shared notes: `reports/` in the coordination folder (run requests, review campaign
  `PR_REVIEW.md`, plans). Edit only your own rows or sections.
- Branches: `integration` collects reviewed pull requests and is measured as a whole; `main`
  moves when `integration` shows no unjustified regression.
- Upstream code is taken with `git cherry-pick -x` of the commits that belong to the change,
  never by merging a branch that carries unrelated upstream commits.

## Commits and pull requests

- Signed commits, one subject each. Title `Area: what changed`. The body says why, what was
  measured, and what was not checked.
- Keep authors. Work taken from elsewhere names its source.
- No game files, code or assets in the tree. No game name in code, commits, issues or pull
  requests: write "a retail game".
- On GitHub, mention a contributor (`@name`) only to ask them something that only they can
  answer, in English, once. Do not post repetitive comments.
- Closing a pull request always comes with a clear, constructive reason. Upstream pull requests
  are never modified or closed.
