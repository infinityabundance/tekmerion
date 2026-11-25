#!/usr/bin/env bash
# cap.sh -- run a command under a hard address-space cap.
#
# Tekmerion rule (AGENTS.md §7a): anything that can allocate more than a few GB
# (HOL Light proof builds, native compilation, full sweeps) MUST run through
# this wrapper. Three OOM crashes came from running proof builds uncapped; the
# cap makes such a failure local to the child instead of the whole machine.
#
# Usage: tools/cap.sh [--timeout <secs>] <MEM> -- <command> [args...]
#   e.g. tools/cap.sh 12G -- make -j1 HOLDIR=... curve25519/bignum_add_p25519.correct
#        tools/cap.sh --timeout 600 12G -- make -j1 HOLDIR=... <target>.correct
#
# WHAT IS GUARANTEED
#   * Per process. Every process in the tree inherits RLIMIT_AS = <MEM> via
#     `ulimit -v`, so no single process can exceed the limit. The mechanism was
#     verified to bite on this host.
#   * One group, one kill. The command runs in its own session/process group
#     (`setsid`). With --timeout the whole group receives TERM and then KILL on
#     expiry, not just the leaf. Without --timeout the group is still isolated,
#     but nothing kills it for you.
#
# WHAT IS NOT GUARANTEED
#   * Aggregate memory of the tree. RLIMIT_AS is per-process: a tree of N
#     processes can still consume about N * <MEM>. There is no aggregate cap
#     here. cgroups would provide one, but on this host `systemd-run -p
#     MemoryMax=` is accepted and not applied (verified: the scope reports
#     MemoryMax=infinity and a 2 GB allocation succeeds under a 256 M setting),
#     so the cgroup route is unavailable. The group kill is the strongest
#     available "stop the whole tree" mechanism, and it is time-based, not
#     memory-based. A descendant that deliberately starts its own session
#     (`setsid`) leaves the group, so neither the group kill nor the group RSS
#     sample includes it; common build tools do not do this.
#
# REPORTING
#   A single summary line is printed on stderr:
#     cap.sh: group=<pgid> rc=<rc> sampled_group_peak_rss_kb=<n> max_process_vmhwm_kb=<n>
#   `sampled_group_peak_rss_kb` is the maximum, over samples (0.5 s), of the sum
#   of RSS for processes whose PGID is <pgid>. It can miss a short peak. The
#   per-process `VmHWM` high-water mark is exact for processes observed alive.
#   `/usr/bin/time -v` is ABSENT on this host; where it exists it reports the
#   direct child's `ru_maxrss`, not the tree's, so it is never treated as tree
#   accounting.
#
# Enforcement is RLIMIT_AS via `ulimit -v`. Fail-closed: bad arguments or a
# missing mechanism exit non-zero and run nothing. A raised cap is a decision,
# recorded in the brief that asks for it -- never a convenience.

set -u

usage() {
  echo "usage: tools/cap.sh [--timeout <secs>] <MEM> -- <command> [args...]" >&2
  echo "  MEM is e.g. 512M, 8G, 12G (suffix K/M/G, optional trailing B)" >&2
  echo "  --timeout <secs> kills the command's whole process group on expiry" >&2
  exit 2
}

timeout_secs=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --timeout)
      [ "$#" -ge 2 ] || usage
      timeout_secs="$2"
      shift 2
      ;;
    --timeout=*)
      timeout_secs="${1#--timeout=}"
      shift
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "cap.sh: unknown option '$1'" >&2
      usage
      ;;
    *)
      break
      ;;
  esac
done

[ "$#" -ge 3 ] || usage
mem="$1"
shift
[ "$1" = "--" ] || usage
shift

case "$timeout_secs" in
  '') ;;
  *[!0-9]*)
    echo "cap.sh: bad timeout '$timeout_secs' (integer seconds)" >&2
    exit 2
    ;;
  *)
    [ "$timeout_secs" -gt 0 ] || {
      echo "cap.sh: timeout must be positive" >&2
      exit 2
    }
    ;;
esac

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

command -v setsid >/dev/null 2>&1 || {
  echo "cap.sh: setsid not found; refusing to run without group isolation" >&2
  exit 3
}

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/cap.XXXXXX")" || {
  echo "cap.sh: cannot create a temporary directory" >&2
  exit 3
}
pgfile="$tmpdir/pgid"
flagfile="$tmpdir/timedout"
peakfile="$tmpdir/peak"
hwmfile="$tmpdir/hwm"
: > "$peakfile"
: > "$hwmfile"
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

pgid=""
on_signal() {
  if [ -n "$pgid" ]; then
    kill -TERM -- "-$pgid" 2>/dev/null || true
  fi
  exit 130
}
trap on_signal INT TERM

# New session => new process group. `setsid -w` waits for the command even if
# setsid itself had to fork. The inner shell records its own PGID before it
# execs the command; every descendant inherits both the group and RLIMIT_AS.
# Keep the caller's stdin when it is open (an async command would otherwise get
# /dev/null); fall back to /dev/null rather than failing on a closed fd 0.
if [ -e /dev/fd/0 ]; then
  setsid -w bash -c '
    pgfile=$1; kb=$2; shift 2
    echo "$BASHPID" > "$pgfile"
    ulimit -v "$kb" || { echo "cap.sh: ulimit failed" >&2; exit 3; }
    exec "$@"
  ' _ "$pgfile" "$kb" "$@" 0<&0 &
else
  setsid -w bash -c '
    pgfile=$1; kb=$2; shift 2
    echo "$BASHPID" > "$pgfile"
    ulimit -v "$kb" || { echo "cap.sh: ulimit failed" >&2; exit 3; }
    exec "$@"
  ' _ "$pgfile" "$kb" "$@" </dev/null &
fi
child=$!

tries=0
while [ ! -s "$pgfile" ] && [ "$tries" -lt 100 ]; do
  kill -0 "$child" 2>/dev/null || break
  sleep 0.02
  tries=$((tries + 1))
done
if [ -s "$pgfile" ]; then
  pgid="$(cat "$pgfile")"
fi

# Sample the group's RSS and each member's VmHWM. Sampling cannot prove the
# absence of a shorter peak; the labels say "sampled" for that reason.
sampler=""
if [ -n "$pgid" ]; then
  (
    peak=0
    hwm=0
    while kill -0 "$pgid" 2>/dev/null; do
      rss=$(ps -eo pgid=,rss= 2>/dev/null |
            awk -v g="$pgid" '$1==g {s+=$2} END {print s+0}')
      [ -n "$rss" ] || rss=0
      if [ "$rss" -gt "$peak" ]; then peak="$rss"; fi
      for p in $(ps -eo pgid=,pid= 2>/dev/null |
                 awk -v g="$pgid" '$1==g {print $2}'); do
        h=$(awk '/^VmHWM:/ {print $2}' "/proc/$p/status" 2>/dev/null)
        if [ -n "$h" ] && [ "$h" -gt "$hwm" ]; then hwm="$h"; fi
      done
      printf '%s\n' "$peak" > "$peakfile"
      printf '%s\n' "$hwm" > "$hwmfile"
      sleep 0.5
    done
  ) &
  sampler=$!
fi

# Timeout: signal the whole group, then escalate. The flag records that the
# expiry happened, so the final status is 124 even if the child exited on TERM.
wd=""
if [ -n "$timeout_secs" ] && [ -n "$pgid" ]; then
  (
    sleep "$timeout_secs"
    : > "$flagfile"
    kill -TERM -- "-$pgid" 2>/dev/null || true
    sleep 1
    kill -KILL -- "-$pgid" 2>/dev/null || true
  ) &
  wd=$!
fi

rc=0
wait "$child" || rc=$?

if [ -n "$wd" ]; then
  if [ -f "$flagfile" ]; then
    wait "$wd" 2>/dev/null || true
  else
    kill "$wd" 2>/dev/null || true
    wait "$wd" 2>/dev/null || true
  fi
fi
if [ -n "$sampler" ]; then
  kill "$sampler" 2>/dev/null || true
  wait "$sampler" 2>/dev/null || true
fi

if [ -f "$flagfile" ]; then
  echo "cap.sh: TIMEOUT after ${timeout_secs}s; process group ${pgid} signalled (TERM then KILL)" >&2
  rc=124
fi

peak=0
hwm=0
[ -s "$peakfile" ] && peak="$(cat "$peakfile")"
[ -s "$hwmfile" ] && hwm="$(cat "$hwmfile")"
echo "cap.sh: group=${pgid:-none} rc=${rc} sampled_group_peak_rss_kb=${peak:-0} max_process_vmhwm_kb=${hwm:-0}" >&2

exit "$rc"
