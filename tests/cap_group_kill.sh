#!/usr/bin/env bash
# XVI.E3: the cap's process-group kill control.
#
# The cap itself is per-process (RLIMIT_AS). This control shows the part that
# *is* available for the tree: the command runs in its own process group, and
# `--timeout` signals the whole group. A parent spawns a child that tries to
# allocate past the cap; the child dies of the cap, the parent ignores that and
# keeps sleeping, and the timeout must leave neither process alive.
#
# Usage: tests/cap_group_kill.sh
#   CAP_CONTROL_MEM (default 512M)   per-process cap
#   CAP_CONTROL_TIMEOUT (default 3)  whole-group timeout in seconds
#
# Exit 0 iff cap.sh returns 124, the child's status is recorded, and both PIDs
# are gone afterwards.

set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

mem="${CAP_CONTROL_MEM:-512M}"
secs="${CAP_CONTROL_TIMEOUT:-3}"

tmp="$(mktemp -d)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

# The child allocates 4 GiB in one object. Under the 512 MiB cap OCaml reports
# `Out of memory during evaluation.` and exits 2; it is not killed by the
# kernel, which is the point: the cap bites, and the parent is unaffected.
cat > "$tmp/alloc.ml" <<'OCAML'
let _ = Bytes.make (4 * 1024 * 1024 * 1024) (Char.chr 120)
OCAML

cat > "$tmp/parent.sh" <<'PARENT'
#!/usr/bin/env bash
set -u
pidfile=$1
statusfile=$2
childprog=$3
echo "PARENT_PID=$$" > "$pidfile"
trap 'echo "PARENT_TERM=143" >> "$statusfile"; exit 143' TERM
ocaml "$childprog" &
child=$!
echo "CHILD_PID=$child" >> "$pidfile"
wait "$child"
rc=$?
echo "CHILD_STATUS=$rc" >> "$statusfile"
echo "PARENT_SLEEPING=$$" >> "$statusfile"
sleep 300
PARENT
chmod +x "$tmp/parent.sh"

set +e
tools/cap.sh --timeout "$secs" "$mem" -- \
  bash "$tmp/parent.sh" "$tmp/pids" "$tmp/status" "$tmp/alloc.ml"
rc=$?
set -e

parent="$(sed -n 's/^PARENT_PID=//p' "$tmp/pids" | head -1)"
child="$(sed -n 's/^CHILD_PID=//p' "$tmp/pids" | head -1)"

echo "CONTROL cap_rc=$rc"
echo "CONTROL parent_pid=$parent child_pid=$child"
echo "CONTROL recorded:"
sed 's/^/CONTROL   /' "$tmp/status" 2>/dev/null || true

alive=0
for p in "$parent" "$child"; do
  if [ -n "$p" ] && kill -0 "$p" 2>/dev/null; then
    alive=$((alive + 1))
  fi
done
echo "CONTROL alive_after=$alive"
echo "CONTROL ps_after:"
ps -o pid,pgid,stat,comm -p "${parent},${child}" 2>/dev/null || true

child_status="$(sed -n 's/^CHILD_STATUS=//p' "$tmp/status" | head -1)"
if [ "$rc" -eq 124 ] && [ "$alive" -eq 0 ] && [ "${child_status:-}" = "2" ]; then
  echo "CONTROL PASS"
  exit 0
fi
echo "CONTROL FAIL"
exit 1
