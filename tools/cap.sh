#!/usr/bin/env bash
# cap.sh -- run a command under a hard address-space cap.
#
# Tekmerion rule (AGENTS.md §7): anything that can allocate more than a few GB
# (HOL Light proof builds, native compilation, full sweeps) MUST run through
# this wrapper. Three OOM crashes came from running proof builds uncapped; the
# cap makes such a failure local to the child instead of the whole machine.
#
# Usage: tools/cap.sh <MEM> -- <command> [args...]
#   e.g. tools/cap.sh 12G -- make -j1 HOLDIR=... curve25519/bignum_add_p25519.correct
#
# Enforcement is RLIMIT_AS via `ulimit -v`, because on this host
# `systemd-run -p MemoryMax=` is accepted but not applied (verified: the scope
# reports MemoryMax=infinity and a 2 GB allocation succeeds under a 256 M
# setting). `ulimit -v` was verified to bite.
#
# Fail-closed: bad arguments or a missing mechanism exit non-zero and run
# nothing. A raised cap is a decision, recorded in the brief that asks for it —
# never a convenience.

set -u

usage() {
  echo "usage: tools/cap.sh <MEM> -- <command> [args...]" >&2
  echo "  MEM is e.g. 512M, 8G, 12G (suffix K/M/G, optional trailing B)" >&2
  exit 2
}

[ "$#" -ge 3 ] || usage
mem="$1"
shift
[ "$1" = "--" ] || usage
shift

num=$(printf '%s' "$mem" | sed 's/[BbKkMmGg]*$//')
unit=$(printf '%s' "$mem" | sed 's/^[0-9]*//' | tr 'A-Z' 'a-z' | sed 's/b$//')
case "$num" in ''|*[!0-9]*) echo "cap.sh: bad memory limit '$mem'" >&2; exit 2 ;; esac
case "$unit" in
  ''|k) kb=$((num)) ;;
  m) kb=$((num * 1024)) ;;
  g) kb=$((num * 1024 * 1024)) ;;
  *) echo "cap.sh: bad memory unit in '$mem'" >&2; exit 2 ;;
esac
[ "$kb" -gt 0 ] || { echo "cap.sh: limit must be positive" >&2; exit 2; }

# Sanity: prove the mechanism works in this shell before trusting it.
if ! ( ulimit -v "$kb" 2>/dev/null ); then
  echo "cap.sh: cannot set RLIMIT_AS to ${kb}K; refusing to run uncapped" >&2
  exit 3
fi

exec bash -c 'ulimit -v "$1" || { echo "cap.sh: ulimit failed" >&2; exit 3; }; shift; exec "$@"' \
  _ "$kb" "$@"
