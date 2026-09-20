# Paste into every agent brief (after the task itself)

## Checkpoints — not optional

You may be killed at any moment by a session limit, and everything that is only in your
context is lost. So:

1. **Before anything else**, create `docs/research/wip/<task-slug>.md` in your worktree with
   the task in one line and an empty "Findings / Decisions / Dead ends / Next step" skeleton,
   and commit it.
2. **After every finding** (a measured fact, a failed approach, a design decision) append
   one line to that file. Not at the end — at the moment you learn it.
3. **After every completed step** commit on your branch (message: one full imperative
   sentence ending with a period, plus the attribution line). Work-in-progress commits are
   fine; they are squashed at merge.
4. **If your worktree already has that file or commits when you start, you are resuming**:
   read the file and `git log`, then continue from "Next step". Never start over.
5. Every edit you make is also snapshotted to `refs/autosave/<branch>` by a hook; you do not
   need to do anything for that, but know it exists.

When you report, the wip file's durable content must already be in the experiment
backlog or the relevant spec; delete the wip file in your last commit.
