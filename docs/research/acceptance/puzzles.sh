#!/bin/bash
# Acceptance test: a real third-party S60 application through symdev with no edit
# inside the project (experiment 67).
#
#   docs/research/acceptance/puzzles.sh [build|package|run|all]     (default: all)
#
# Clones Simon Tatham's Puzzles (S60 port) into a scratch directory, copies it, adds
# exactly one file — puzzles-symdev.toml beside this script — and drives symdev over it.
# Afterwards it diffs the built tree against the pristine clone: any difference outside
# `build/` and the added `symdev.toml` fails the test, because editing the project is
# precisely what this is here to rule out.
#
# Nothing from the cloned project is ever copied into this repository.
#
# Needs: SYMDEV_EPOCROOT, SYMDEV_GXX, SYMDEV_LD, SYMDEV_GCC_LIB, SYMDEV_GCC_TARGET_LIB
# and, for `run`, SYMDEV_EKA2L1. `run` leaves the emulator running; its pid is in
# build/eka2l1.pid and it ignores SIGTERM, so stop it with `kill -9`.
set -u

REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
SCRATCH=${SYMDEV_ACCEPTANCE_DIR:-${TMPDIR:-/tmp}/symdev-acceptance}
PRISTINE="$SCRATCH/puzzless60"
WORK="$SCRATCH/work"
SYMDEV=${SYMDEV_BIN:-$REPO/target/debug/symdev}
STAGE=${1:-all}
: "${SYMDEV_SIGN_PASSWORD:=secret}"
export SYMDEV_SIGN_PASSWORD

mkdir -p "$SCRATCH"
if [ ! -d "$PRISTINE/.git" ]; then
    echo "== cloning puzzless60 into $PRISTINE"
    git clone --depth 1 -q https://github.com/tdionizio/puzzless60.git "$PRISTINE" || exit 1
fi
[ -x "$SYMDEV" ] || { echo "no symdev binary at $SYMDEV (cargo build -p symdev-cli --bins)"; exit 1; }

rm -rf "$WORK"
cp -r "$PRISTINE" "$WORK"
rm -rf "$WORK/.git"
cp "$(dirname "${BASH_SOURCE[0]}")/puzzles-symdev.toml" "$WORK/symdev.toml"

cd "$WORK" || exit 1
status=0
for stage in build package run; do
    case "$STAGE" in all) ;; "$stage") ;; *) continue ;; esac
    echo "== symdev $stage"
    "$SYMDEV" "$stage" || { status=1; break; }
done

# The point of the test: the project itself is untouched.
untouched=$(diff -rq "$PRISTINE" "$WORK" -x .git -x build -x symdev.toml)
if [ -n "$untouched" ]; then
    echo "FAIL: the project was modified:"
    echo "$untouched"
    exit 1
fi
[ $status -eq 0 ] && echo "== the project is unmodified"
exit $status
