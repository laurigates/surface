---
name: eli5
description: Explain something in plain, non-technical language — "explain like I'm 5". Use when the user asks to ELI5, "explain simply", "in plain English", "what did we just do", or wants a jargon-free version of a PR, diff, commit, code, error, or concept.
---

# eli5

Explain the target so a smart person with **no context on this codebase** gets it. Optimize for "now I understand," not for completeness.

## Figure out the target

- If the user names it (a PR, file, function, error, concept), explain that.
- If they say "what did we just do" / "the PR we just did" with no target, explain the most recent thing from this conversation (the last PR, commit, or change).
- If genuinely ambiguous, ask one short question — otherwise just pick the obvious target and go.

## How to explain

- **Lead with the point in one sentence.** What is it, in everyday terms?
- **Use a concrete analogy** (a guard at a door, a helpful shopper, a librarian). Map the real pieces onto it.
- **Plain words over jargon.** If a technical term is unavoidable, define it inline the first time in parentheses.
- **Short.** A few sentences to a few small paragraphs. A short bulleted list of "the moving parts" is fine. No walls of text.
- **Say why it matters** — what's better now, or what problem it solves.

## Don'ts

- No code blocks unless the user asks. The point is to avoid needing to read code.
- Don't restate the diff line by line — that's the opposite of ELI5.
- Don't condescend. "Simple" means clear, not childish.

## Tone

Warm and direct. Imagine explaining to a sharp colleague from a different field over coffee.
