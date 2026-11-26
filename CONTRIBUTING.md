# Contributing to Victory Native

Pull requests are welcome from anyone, on any part of the project: shaders, memory, ray
tracing, audio, synchronisation, streaming, system APIs, tools, Linux, docs. Every pull
request gets a review and a decision, with the reason.

## The rule: translate, don't fake

- Translate the behaviour of the Xbox hardware and APIs into its PC equivalent. Where none
  exists, reproduce the semantics.
- Never invent a result to hide a feature that is wrong or missing. That rules out stand-in
  light, made-up values, return codes or success answers, empty or constant textures, and
  dropped work passed off as done.
- Never present a visual enhancement as a renderer fix. Brightening, clamping, blurring or
  masking a frame does not fix it.
- Fix root causes. A temporary workaround is acceptable only to diagnose or to unblock an
  investigation. It must be optional and off by default, listed in
  [docs/workarounds.md](docs/workarounds.md), and stated in the log whenever it acts.
- Optimise the implementation without changing what the game is meant to do. A faster path
  must give the same result.
- Call a problem fixed only when measurements show it.
- Never hide an error, a crash, an incompatibility or a regression. Report it, log it, list it.

## What a pull request needs

- **One subject**, and a title of the form `Area: what changed`.
- **Measurements, before and after.** Say what you ran: card, CPU, RAM, OS, console profile,
  ray tracing on or off, switches, scene. Give the frame time, and for anything that can touch
  the picture, screenshots or a short clip cropped to the game's picture. A change that alters
  the picture stays behind an off-by-default switch until it is shown identical.
- **Tests** for logic that runs on the CPU. These must pass:
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --locked` and
  `cargo xtask policy`.
- **Honest scope.** Say what was not checked.
- **No game files, code or assets**, and no game name in code, commits, issues or pull
  requests. Write "a retail game".

## How a pull request is decided

| status | meaning |
|---|---|
| MERGE | correct, useful, and validated by its tests and a run |
| NEEDS TESTING | promising; it needs a run or a test we have not done yet |
| NEEDS CHANGES | worth having; the review says what to change |
| NEEDS AUTHOR INPUT | a question for the author, who is asked by name |
| DUPLICATE | covered by another pull request or by code already merged |
| REJECTED | incorrect, not needed, or against the rule above. The reason is always given. |

Decisions rest on the code, the tests and runs, not on the description alone. Pull requests
go to `main`. One is merged when its tests pass and a run shows nothing regressing in picture,
stability or frame time, or when a regression is justified and stated.

## Upstream

Victory Native goes its own way and is developed here, not upstream. The commits and pull
requests of [VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX) are still read,
and taken when they fit the rule above. Its open pull requests are reviewed here; the tracking file is `reports/PR_REVIEW.md` in the coordination notes.
The ones we take are opened again here, with their title, description and commit authors
kept and a link to the original. Their authors are welcome to carry them on here. The upstream
pull requests themselves are never modified or closed by us.

## License

GPL-3.0-or-later. By contributing, you agree that your work is published under it. Commits
keep their authors. Work taken from elsewhere names its source commit (`git cherry-pick -x`).
