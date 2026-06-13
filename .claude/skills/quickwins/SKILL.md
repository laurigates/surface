---
name: quickwins
description: Scan a milestone (or the backlog / open issues) for low-effort, high-value work, triage genuine quick wins from disguised epics, and recommend what to do next. Use when the user asks for "quick wins", "low-hanging fruit", "what's easy in milestone X", or "anything I can knock out".
---

# quickwins

Find work that is **small to do and worth doing**, separate it from work that only *looks* small, and recommend a next step.

## Gather the candidates

- A milestone named → list its open issues: `gh issue list --milestone "<name>" --state open --limit 50`. (Milestone titles, not numbers, for this flag; `gh api repos/:owner/:repo/milestones` if you need to find the title.)
- No milestone named → ask which milestone, or fall back to all open issues.
- Read the full body of each plausible candidate (`gh issue view <n>`, and `--comments` when there's discussion) before judging. A one-line title hides both trivial fixes and design epics.

## Triage: quick win vs disguised epic

A **quick win** is: contained to a few files, low blast radius, clear acceptance, and lands green without a migration or design debate. Examples: a CI/config addition, a focused test file, a docs fix, a small self-contained feature.

**Not** a quick win (flag these, don't start them):
- Issues whose own body/comments describe a multi-step design (versioned formats, migrations, validation harnesses, cross-cutting refactors). A long comment thread is a strong tell.
- Anything that says "may surface real bugs", "budget for follow-ups", or otherwise has open-ended scope.
- Work that needs a decision the user hasn't made.

When something looks small but isn't, say so in one line and explain why — that's as useful as the win itself.

## Recommend, then act

- Lead with a short ranked list: the genuine quick wins first, then a one-line dismissal of the near-misses with the reason.
- Recommend the single best next one and say why (highest value-to-effort, lowest risk).
- If the user says go (or asked you to "do" the quick wins), implement the top one end-to-end and open a PR via the `create-pr` skill — branch, gates, Co-Authored-By + 🤖 footers, `Closes #N`, report mergeable, don't merge.
- Do one at a time; confirm before moving to the next unless told to batch.

## Output

A tight triage: what's quick (and why), what isn't (and why), and your recommendation. No exhaustive survey of options you won't pursue.
