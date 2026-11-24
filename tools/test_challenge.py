#!/usr/bin/env python3
"""Negative tests for tools/challenge.py (Ordeal O1.1/O1.2 firing controls)."""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import challenge  # noqa: E402

TASK = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT"
STATEMENT = "`!z x y m n pc. foo ==> bar`"


def _tree(root: pathlib.Path) -> dict:
    s2n = root / "s2n"
    obj = s2n / "x86" / "curve25519" / "bignum_add_p25519.o"
    obj.parent.mkdir(parents=True)
    obj.write_bytes(b"\x7fELF-object-bytes")
    corpus = root / "problems.json"
    corpus.write_text(json.dumps({
        TASK: {"category": "functional_correctness_x86", "query": STATEMENT},
    }))
    return {"s2n": s2n, "problems": corpus, "obj": obj}


def _freeze(root, paths, out=None):
    out = out or (root / "c.challenge")
    assert challenge.main([
        "freeze", "--task", TASK, "--out", str(out),
        "--problems", str(paths["problems"]),
        "--s2n-bignum", str(paths["s2n"]),
    ]) == 0
    return out


def _verify(paths, chal, evaldir=None, extra=()):
    argv = ["verify", "--challenge", str(chal),
            "--problems", str(paths["problems"]),
            "--s2n-bignum", str(paths["s2n"])]
    if evaldir is not None:
        argv += ["--evaldir", str(evaldir)]
    argv += list(extra)
    return challenge.main(argv)


def _judged(root) -> pathlib.Path:
    evaldir = root / "eval"
    evaldir.mkdir()
    ml = evaldir / "ml_files_x86_bignum_add_p25519.ml"
    ml.write_text(
        'bench_run "' + TASK + '" (' + STATEMENT + ') (fun () -> TAC) 1 "x";\n'
    )
    (evaldir / f"{TASK}.judge.txt").write_text("OK")
    return evaldir


def test_round_trip_passes(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    evaldir = _judged(tmp_path)
    capsys.readouterr()
    assert _verify(p, chal, evaldir) == 0
    assert "RESULT PASS" in capsys.readouterr().out


def test_tampered_statement_is_caught(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    raw = chal.read_bytes()
    # Change one byte inside the statement block.
    i = raw.index(challenge.BEGIN) + len(challenge.BEGIN)
    chal.write_bytes(raw[:i] + b"X" + raw[i + 1:])
    capsys.readouterr()
    assert _verify(p, chal) == 1
    out = capsys.readouterr().out
    assert "FAIL statement-sha256" in out
    assert "FAIL statement==problems.json" in out


def test_tampered_object_is_caught(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    p["obj"].write_bytes(b"\x7fELF-object-bytez")
    capsys.readouterr()
    assert _verify(p, chal) == 1
    assert "FAIL object-sha256" in capsys.readouterr().out


def test_judge_program_with_wrong_statement_is_caught(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    evaldir = _judged(tmp_path)
    ml = evaldir / "ml_files_x86_bignum_add_p25519.ml"
    ml.write_text('bench_run "' + TASK + '" (`!z. other`) (fun () -> TAC) 1 "x";\n')
    capsys.readouterr()
    assert _verify(p, chal, evaldir) == 1
    assert "FAIL judge-embeds-statement" in capsys.readouterr().out


def test_judge_program_named_for_a_shared_template(tmp_path, capsys):
    """A task may be inlined into a differently-named template (observed:
    arm.bignum_sub_p256 lives in ml_files_arm_p256_montjdouble.ml)."""
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    evaldir = tmp_path / "eval"
    evaldir.mkdir()
    (evaldir / "ml_files_x86_some_other_template.ml").write_text(
        'bench_run "' + TASK + '" (' + STATEMENT + ') (fun () -> TAC) 1 "x";\n')
    (evaldir / f"{TASK}.judge.txt").write_text("OK")
    capsys.readouterr()
    assert _verify(p, chal, evaldir) == 0
    assert "RESULT PASS" in capsys.readouterr().out


def test_post_hoc_is_refused_when_pre_search_required(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = tmp_path / "c.challenge"
    assert challenge.main([
        "freeze", "--task", TASK, "--out", str(chal),
        "--problems", str(p["problems"]), "--s2n-bignum", str(p["s2n"]),
        "--post-hoc", "recorded after the run",
    ]) == 0
    capsys.readouterr()
    assert _verify(p, chal, extra=("--require-pre-search",)) == 1
    assert "FAIL pre-search" in capsys.readouterr().out


def test_duplicate_header_key_is_refused(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    raw = chal.read_bytes()
    i = raw.index(challenge.BEGIN)
    header = raw[:i].replace(b"task " + TASK.encode(), b"task " + TASK.encode(), 1)
    header = header + b"task " + TASK.encode() + b"\n"
    chal.write_bytes(header + raw[i:])
    capsys.readouterr()
    assert _verify(p, chal) == 2
    assert "duplicate header key" in capsys.readouterr().err


def test_vacuous_wrong_task_is_refused(tmp_path, capsys):
    p = _tree(tmp_path)
    chal = _freeze(tmp_path, p)
    corpus = json.loads(p["problems"].read_text())
    corpus["x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT"] = corpus.pop(TASK)
    p["problems"].write_text(json.dumps(corpus))
    capsys.readouterr()
    assert _verify(p, chal) == 1
    assert "FAIL task-present" in capsys.readouterr().out
