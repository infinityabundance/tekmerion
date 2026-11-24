#!/usr/bin/bash
# Host launcher for the O10.2 structural container run.
# Usage: bash .bench/run-sealed-court-v3.sh [preflight|full]
set -u

MODE="${1:-full}"
REPO=/mnt/1tb_kingston/Tekmerion
COURT="$REPO/.bench/sealed-court-v3"
OUT="$REPO/.bench/sealed-court-v3-out"
mkdir -p "$OUT"

docker run --rm --network none --read-only \
  --tmpfs /tmp:rw,size=256m \
  --tmpfs /work:rw,exec,size=2g \
  --user 1000:1000 \
  --env HOME=/work/home \
  --env MODE="$MODE" \
  --volume /usr:/usr:ro \
  --volume /lib64:/lib64:ro \
  --volume /etc:/etc:ro \
  --volume "$COURT:/court:ro" \
  --volume /mnt/1tb_kingston/Tekmerion/.bench/hol-light:/mnt/1tb_kingston/Tekmerion/.bench/hol-light:ro \
  --volume /home/one/.elan/toolchains/leanprover--lean4---v4.29.1:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1:ro \
  --entrypoint /usr/bin/bash \
  ubuntu:24.04 /court/run-inside.sh

echo "docker_exit=$?"
