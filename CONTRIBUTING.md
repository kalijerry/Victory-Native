# Contributing to Victory AMD

Pull requests are welcome, from anyone, on any part: shaders, memory, ray tracing, tools,
Linux, docs. Each one gets a review and a decision with its reason.

## What we look for

- **The title's own picture.** No stand-in light, clamp, blur or mask that makes a frame
  look better than what the title draws. Fix the cause, or document the defect in
  [docs/workarounds.md](docs/workarounds.md).
- **Measured.** Say what you ran (card, CPU, RAM, OS, profile, ray tracing, switches, scene)
  and what changed, before and after: frame time and the picture (screenshots, or a short
  clip cropped to the game's picture). A change that alters the picture stays behind an
  off-by-default switch until it is shown identical.
- **Tests** for logic that can be tested on the CPU, and `cargo clippy --workspace
  --all-targets --locked -- -D warnings`, `cargo test --locked` and `cargo xtask policy`
  passing.
- **One subject per pull request.** Title in the form `Area: what changed`.
- **No game files, code or assets,** and no game name in code, commits or pull requests
  ("a retail game").

## How a pull request is decided

| status | meaning |
|---|---|
| MERGE | correct, useful, validated by tests and a run |
| NEEDS TESTING | promising; needs a run or a test we have not done yet |
| NEEDS CHANGES | worth having; the review says what to change |
| NEEDS AUTHOR INPUT | a question for the author |
| DUPLICATE | covered by another pull request or by code already merged |
| REJECTED | incorrect, not needed, or against the rules above (the reason is always given) |

Pull requests go to `integration` first. They are measured there together, then `integration`
goes to `main` when nothing regresses in picture, stability or frame time.

## Pull requests from upstream

The open pull requests of [VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)
are reviewed here (`reports/PR_REVIEW.md`). The ones we take are opened again here with
their title, description, commit authors and a link to the original. Their authors are
welcome to carry them on here. We never touch the upstream pull requests themselves.

License: GPL-3.0-or-later. By contributing you agree your work is published under it.
