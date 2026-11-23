#!/usr/bin/env python3
"""w4_close.py -- deterministic closer: witness -> arithmetic-closing tactics.

The closer has two layers:

  simulation(w)      the ``ARM_ACCSTEPS_TAC`` chain, over exactly the derived
                     accumulate set.  For ``sub`` it splits at the decisive
                     borrow flag so the flag meaning can be injected between
                     the two carry chains (the split point is derived from
                     ``w.flag_meaning.source_steps``).
  post_simulation(w)  the congruence application and its side conditions,
                     selected by ``w.congruence.principle`` and filled from
                     the witness terms.

No external solver is used.  Every fragment that is still transcribed from the
human close is listed in ``DEVELOPMENT_ONLY`` and is also recorded in the
witness and the emission receipt.  The congruence *principle*, the *congruence
term*, the *flag meaning*, the bounds and the accumulate set all come from the
witness, i.e. they are derived, not copied.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Dict, List

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from witness import Witness  # noqa: E402


# Human-informed fragments.  Each entry says exactly what is transcribed and
# why it cannot yet be derived.
DEVELOPMENT_ONLY: Dict[str, str] = {
    "simulation.split_for_sub":
        "The sub routine is simulated in two carry chains with the borrow flag "
        "injected between them. The split point is derived from the flag's "
        "source step, but splitting at all follows the human proof.",
    "simulation.step9_cleanup":
        "RULE_ASSUM_TAC(REWRITE_RULE[WORD_UNMASK_64; NOT_LE]) after the CSETM "
        "step rewrites the flag-only step; copied from the human sub proof.",
    "post.bitval_rewrites":
        "RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT; ADD_CLAUSES; "
        "VAL_WORD_BITVAL]) and the WORD_AND_MASK / WORD_UNMASK_64 sets are "
        "copied from the human closes.",
    "post.side_condition_order":
        "The order of the congruence's side conditions (bounds first, case "
        "split last) follows the human proof; HOL does not force it.",
    "post.digit_expansion":
        "BIGNUM_LEXPAND_CONV / BIGNUM_EXPAND_CONV with the hand-picked "
        "LAND_CONV/RAND_CONV traversal is the human expansion idiom.",
    "post.accumulator_close":
        "DECARRY_RULE/DESUM_RULE then BOUNDER_TAC/REAL_INTEGER_TAC is the "
        "human arithmetic close.",
}


def _joinsemi(nums: List[int]) -> str:
    return "[" + ";".join(str(n) for n in nums) + "]"


def _rng(lo: int, hi: int) -> str:
    return f"({lo}--{hi})"


def _exp_vars(w: Witness) -> str:
    return "[" + "; ".join(f'"{v.value}"' for v in w.inputs) + "]"


# --------------------------------------------------------------------------
# Simulation
# --------------------------------------------------------------------------


def simulation(w: Witness) -> str:
    lo, hi = w.simulation_range
    if w.op != "sub":
        return (f"ARM_ACCSTEPS_TAC {w.exec_name} "
                f"{_joinsemi(w.accumulate)} {_rng(lo, hi)}")

    # Split at the decisive borrow flag's write step.
    split = max(w.flag_meaning.source_steps)
    head = [n for n in w.accumulate if n <= split]
    tail = [n for n in w.accumulate if n > split + 1]
    plain = split + 1
    middle = _borrow_subgoal(w)
    return "\n".join([
        f"ARM_ACCSTEPS_TAC {w.exec_name} {_joinsemi(head)} {_rng(lo, split)} THEN",
        middle + " THEN",
        f"ARM_STEPS_TAC {w.exec_name} [{plain}] THEN",
        "RULE_ASSUM_TAC(REWRITE_RULE[WORD_UNMASK_64; NOT_LE]) THEN",
        f"ARM_ACCSTEPS_TAC {w.exec_name} {_joinsemi(tail)} "
        f"{_rng(plain + 1, hi)}",
    ])


def _borrow_subgoal(w: Witness) -> str:
    fm = w.flag_meaning
    if fm.flag is None or fm.bound is None:
        raise ValueError("sub witness has no named flag")
    return "\n".join([
        f"SUBGOAL_THEN `{fm.flag} <=> {fm.meaning}` SUBST_ALL_TAC THENL",
        f" [MATCH_MP_TAC {fm.rule} THEN EXISTS_TAC `{fm.bound}` THEN",
        f"  MAP_EVERY EXPAND_TAC {_exp_vars(w)} THEN "
        "REWRITE_TAC[GSYM REAL_OF_NUM_ADD] THEN",
        "  REWRITE_TAC[GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN",
        "  ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN",
        "  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN BOUNDER_TAC[];",
        "  ALL_TAC]",
    ])


# --------------------------------------------------------------------------
# Post-simulation close
# --------------------------------------------------------------------------

_COMMON = "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN"


def post_simulation(w: Witness) -> str:
    p = w.congruence.principle
    if p == "EQUAL_FROM_CONGRUENT_MOD_MOD":
        return _congruent_mod_mod(w)
    if p == "EQUAL_FROM_CONGRUENT_REAL":
        return _congruent_real(w)
    if p == "INT_REM_UNIQ":
        return _int_rem_uniq(w)
    raise ValueError(f"no closer for principle {p!r}")


def _congruent_mod_mod(w: Witness) -> str:
    mod = w.congruence.modulus
    c = w.congruence
    fm = w.flag_meaning
    assert fm.flag is not None and fm.bound is not None
    return "\n".join([
        _COMMON,
        "RULE_ASSUM_TAC(REWRITE_RULE",
        " [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN",
        "CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN",
        f"ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{w.final_state}\" THEN",
        f"MATCH_MP_TAC {c.principle} THEN MAP_EVERY EXISTS_TAC",
        f" [`{c.width}`;",
        f"  `{c.witness}`] THEN",
        "REPEAT CONJ_TAC THENL",
        " [CONV_TAC NUM_REDUCE_CONV THEN BOUNDER_TAC[];",
        f"  REWRITE_TAC[{mod}] THEN ARITH_TAC;",
        f"  REWRITE_TAC[{mod}] THEN ARITH_TAC;",
        "  ALL_TAC;",
        "  ASM_SIMP_TAC[MOD_ADD_CASES; GSYM NOT_LE; COND_SWAP] THEN",
        "  COND_CASES_TAC THEN ASM_SIMP_TAC[GSYM REAL_OF_NUM_SUB] THEN",
        f"  REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES; {mod}] THEN REAL_ARITH_TAC] THEN",
        f"SUBGOAL_THEN `{fm.flag} <=> {fm.meaning}` (SUBST1_TAC o SYM) THENL",
        f" [MATCH_MP_TAC {fm.rule} THEN EXISTS_TAC `{fm.bound}` THEN",
        f"  MAP_EVERY EXPAND_TAC {_exp_vars(w)} THEN",
        "  REWRITE_TAC[bignum_of_wordlist; MULT_CLAUSES; ADD_CLAUSES] THEN",
        f"  REWRITE_TAC[GSYM REAL_OF_NUM_EQ; GSYM REAL_OF_NUM_ADD; {mod};",
        "              GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN",
        "  ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN",
        "  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN",
        "  CONV_TAC(BINOP_CONV(BINOP_CONV REAL_POLY_CONV)) THEN BOUNDER_TAC[];",
        f"  MAP_EVERY EXPAND_TAC {_exp_vars(w)} THEN",
        "  ACCUMULATOR_POP_ASSUM_LIST(MP_TAC o end_itlist CONJ o DESUM_RULE) THEN",
        "  REWRITE_TAC[WORD_AND_MASK; GSYM NOT_LE] THEN",
        f"  REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES; bignum_of_wordlist; {mod}] THEN",
        "  COND_CASES_TAC THEN ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN",
        "  CONV_TAC NUM_REDUCE_CONV THEN CONV_TAC WORD_REDUCE_CONV THEN",
        "  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN REAL_INTEGER_TAC]",
    ])


def _congruent_real(w: Witness) -> str:
    mod = w.congruence.modulus
    c = w.congruence
    cs = w.case_split
    sub = w.inputs[0].value if w.inputs else "n"
    # (m - s) MOD m = if s = 0 then 0 else m - s
    shape = f"({mod} - {sub}) MOD {mod} = if {cs.condition} then {cs.true_branch} else {cs.false_branch}"
    guard0 = w.guard[0] if w.guard else ""
    return "\n".join([
        _COMMON,
        "CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN",
        f"ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{w.final_state}\" THEN",
        "REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES] THEN",
        f"MATCH_MP_TAC {c.principle} THEN",
        f"MAP_EVERY EXISTS_TAC [`{c.width}`; `{c.witness}`] THEN",
        "CONJ_TAC THENL [BOUNDER_TAC[]; ALL_TAC] THEN CONJ_TAC THENL",
        f" [UNDISCH_TAC `{guard0}` THEN",
        f"  REWRITE_TAC[REAL_OF_NUM_CLAUSES; {mod}] THEN ARITH_TAC;",
        "  ALL_TAC] THEN",
        "CONJ_TAC THENL [REAL_INTEGER_TAC; ALL_TAC] THEN",
        "",
        f"SUBGOAL_THEN `{shape}`",
        "SUBST1_TAC THENL",
        " [COND_CASES_TAC THEN ASM_REWRITE_TAC[SUB_0; MOD_REFL] THEN",
        "  MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC;",
        "  ONCE_REWRITE_TAC[COND_RAND]] THEN",
        "ASM_SIMP_TAC[GSYM REAL_OF_NUM_SUB] THEN "
        f"EXPAND_TAC \"{sub}\" THEN",
        "REWRITE_TAC[ADD_EQ_0; MULT_EQ_0; EXP_EQ_0; ARITH_EQ] THEN",
        "ACCUMULATOR_POP_ASSUM_LIST(MP_TAC o end_itlist CONJ o DESUM_RULE) THEN",
        "REWRITE_TAC[WORD_UNMASK_64; WORD_SUB_0; VAL_EQ_0] THEN",
        "REWRITE_TAC[WORD_AND_MASK; COND_SWAP; WORD_OR_EQ_0; VAL_EQ_0] THEN",
        "REWRITE_TAC[CONJ_ACI] THEN COND_CASES_TAC THEN",
        f"ASM_REWRITE_TAC[{mod}; GSYM REAL_OF_NUM_CLAUSES; BITVAL_CLAUSES] THEN",
        "CONV_TAC WORD_REDUCE_CONV THEN",
        "DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN REAL_INTEGER_TAC",
    ])


def _int_rem_uniq(w: Witness) -> str:
    mod = w.congruence.modulus
    q = w.congruence.quotient
    if q is None:
        raise ValueError("INT_REM_UNIQ witness has no quotient")
    guard_tac = "MAP_EVERY UNDISCH_TAC [" + "; ".join(
        f"`{g}`" for g in w.guard) + "] THEN"
    return "\n".join([
        _COMMON,
        "CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN",
        f"ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{w.final_state}\" THEN",
        "",
        f"CONV_TAC SYM_CONV THEN MATCH_MP_TAC {w.congruence.principle} THEN",
        f"EXISTS_TAC `{q}` THEN REWRITE_TAC[INT_ABS_NUM] THEN",
        "REWRITE_TAC[INT_ARITH `m - n:int = --b * p + z <=> z = b * p + m - n`] THEN",
        "REWRITE_TAC[int_eq; int_le; int_lt] THEN",
        "REWRITE_TAC[int_add_th; int_mul_th; int_of_num_th; int_sub_th] THEN",
        "REWRITE_TAC[GSYM REAL_OF_NUM_ADD; GSYM REAL_OF_NUM_MUL;",
        "            GSYM REAL_OF_NUM_POW] THEN",
        "MATCH_MP_TAC(REAL_ARITH",
        "`!t:real. p < t /\\",
        "          (&0 <= a /\\ a < p) /\\",
        "          (&0 <= z /\\ z < t) /\\",
        "          ((&0 <= z /\\ z < t) /\\ (&0 <= a /\\ a < t) ==> z = a)",
        "          ==> z = a /\\ &0 <= z /\\ z < p`) THEN",
        f"EXISTS_TAC `(&2:real) pow {w.width_bits}` THEN",
        "",
        f"CONJ_TAC THENL [REWRITE_TAC[{mod}] THEN REAL_ARITH_TAC; ALL_TAC] THEN",
        "CONJ_TAC THENL",
        f" [{guard_tac}",
        "  REWRITE_TAC[GSYM REAL_OF_NUM_LT] THEN",
        f"  ASM_CASES_TAC `{w.case_split.condition}` THEN "
        "ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN",
        f"  POP_ASSUM MP_TAC THEN REWRITE_TAC[{mod}] THEN REAL_ARITH_TAC;",
        "  ALL_TAC] THEN",
        "CONJ_TAC THENL [BOUNDER_TAC[]; STRIP_TAC] THEN",
        "",
        f"MATCH_MP_TAC EQUAL_FROM_CONGRUENT_REAL THEN",
        f"MAP_EVERY EXISTS_TAC [`{w.width_bits}`; `&0:real`] THEN",
        "ASM_REWRITE_TAC[] THEN",
        "CONJ_TAC THENL [REAL_INTEGER_TAC; ALL_TAC] THEN",
        "ACCUMULATOR_POP_ASSUM_LIST(MP_TAC o end_itlist CONJ o DESUM_RULE) THEN",
        "REWRITE_TAC[WORD_AND_MASK] THEN",
        "COND_CASES_TAC THEN ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN",
        "CONV_TAC WORD_REDUCE_CONV THEN",
        f"MAP_EVERY EXPAND_TAC {_exp_vars(w)} THEN "
        "REWRITE_TAC[GSYM REAL_OF_NUM_ADD] THEN",
        f"REWRITE_TAC[GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW; {mod}] THEN",
        "DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN",
        "CONV_TAC(RAND_CONV REAL_POLY_CONV) THEN REAL_INTEGER_TAC",
    ])


def close(w: Witness) -> str:
    """The full post-simulation tactic, ready to append after the simulation."""
    return post_simulation(w)
