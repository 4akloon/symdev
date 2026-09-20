#!/bin/bash
# PostToolUse hook (Write|Edit): snapshot an agent worktree after every file change.
#
# Agents die on session limits with hours of uncommitted work in their worktree. This
# writes the whole working tree, after every edit, to `refs/autosave/<branch>` — a
# separate ref, with a private index — so the branch, HEAD and the agent's own staging
# stay untouched and the agent commits exactly as before. Recovery of a dead agent:
#
#   git -C <worktree> log refs/autosave/<branch>          # what was saved, and when
#   git -C <worktree> checkout refs/autosave/<branch> -- . # put it back on disk
#
# Only worktrees under ~/worktrees/symdev/ are snapshotted; the main checkout is not.
set -u
input=$(cat)
file=$(printf '%s' "$input" | sed -n 's/.*"file_path":"\([^"]*\)".*/\1/p' | head -1)
case "$file" in
  "$HOME"/worktrees/symdev/*) ;;
  *) exit 0 ;;
esac
worktree=$(printf '%s' "$file" | sed -n "s#^\($HOME/worktrees/symdev/[^/]*\).*#\1#p")
[ -d "$worktree/.git" ] || [ -f "$worktree/.git" ] || exit 0
branch=$(git -C "$worktree" symbolic-ref --short -q HEAD) || exit 0
gitdir=$(git -C "$worktree" rev-parse --git-dir) || exit 0
ref="refs/autosave/$branch"
export GIT_INDEX_FILE="$gitdir/autosave-index"
git -C "$worktree" add -A -- . >/dev/null 2>&1 || exit 0
tree=$(git -C "$worktree" write-tree) || exit 0
parent=$(git -C "$worktree" rev-parse -q --verify "$ref" 2>/dev/null)
if [ -n "$parent" ] && [ "$(git -C "$worktree" rev-parse "$parent^{tree}")" = "$tree" ]; then
  exit 0
fi
head=$(git -C "$worktree" rev-parse HEAD)
commit=$(git -C "$worktree" commit-tree "$tree" -p "$head" ${parent:+-p "$parent"} \
  -m "autosave $(date -u +%FT%TZ) after $file") || exit 0
git -C "$worktree" update-ref "$ref" "$commit" >/dev/null 2>&1
exit 0
