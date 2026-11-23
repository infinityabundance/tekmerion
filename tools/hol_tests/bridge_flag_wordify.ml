(* ========================================================================= *)
(* Regression: the flag equation that the bridge case split leaves behind    *)
(* must survive INT_SHIFT_TO_JOIN_CONV.                                      *)
(*                                                                           *)
(* When BRIDGE_SPLIT_CONDITIONS_TAC substitutes a carry bit by the literal 0 *)
(* or 1, the accumulator equation `--2^k * bitval c + val s = ...` becomes   *)
(* `--2^k * &0 + val s = ...` (or `* &1`).  INT_SHIFT_TO_JOIN_CONV used to   *)
(* accept only `&(val x)` and `&(bitval b)` operands, so the whole equation  *)
(* stayed an int atom, BITBLAST left it opaque, and the flag was            *)
(* unconstrained.  The residue then came back SATISFIABLE.                   *)
(*                                                                           *)
(* This file pins that down without the ARM base and without any curve       *)
(* constant.  It FAILS (SAT) on the old conversion and PASSES (UNSAT) once   *)
(* the numeral operand is converted to a word join.                          *)
(*                                                                           *)
(* Run from .bench/s2n-bignum:                                               *)
(*   cat $TEKMERION/tools/hol_tests/bridge_flag_wordify.ml | ../hol-light/hol.sh *)
(* TEKMERION_ROOT is optional; the default is <cwd>/../..                     *)
(* ========================================================================= *)

#load "unix.cma";;
needs "Library/words.ml";;
needs "Cadical/make.ml";;

let root =
  try Sys.getenv "TEKMERION_ROOT"
  with Not_found -> Filename.concat (Sys.getcwd ()) "../..";;

loadt (root ^ "/tools/csat_lib.ml");;
loadt (root ^ "/tools/bridge.ml");;

(* int term constructors: never use `&` in a quotation here, it defaults to *)
(* real_of_num.                                                             *)
let ib n = mk_comb(`int_of_num:num->int`, n);;
let iadd a b = mk_comb(mk_comb(`int_add:int->int->int`, a), b);;
let isub a b = mk_comb(mk_comb(`int_sub:int->int->int`, a), b);;
let imul a b = mk_comb(mk_comb(`int_mul:int->int->int`, a), b);;
let ineg a = mk_comb(`int_neg:int->int`, a);;
let ipow2 k = mk_comb(mk_comb(`int_pow:int->num->int`, ib `2`), k);;
let bit b = mk_comb(`bitval:bool->num`, b);;

let v_c8 = mk_var("c8", `:bool`)
and v_c15 = mk_var("c15", `:bool`)
and v_s16 = mk_var("s16", `:int64`);;
let val64 x = mk_comb(mk_const("val",[mk_finty(Num.num_of_int 64),`:N`]), x);;

(* --2^64 * &lit + &(val s16) = &(bitval c8) - &0 - &(bitval c15) *)
let split_flag lit =
  mk_eq (iadd (imul (ineg (ipow2 `64`)) (ib lit))
              (ib (val64 v_s16)),
         isub (isub (ib (bit v_c8)) (ib `0`)) (ib (bit v_c15)));;

let goal lit concl =
  mk_imp (split_flag lit, concl);;

let le a b = mk_comb(mk_comb(`(<=):num->num->bool`, a), b);;
let num_or a b = mk_comb(mk_comb(`(\/):bool->bool->bool`, a), b);;
let p64 = mk_comb(mk_comb(`(EXP):num->num->num`, `2`), `64`);;
let two64m2 = mk_comb(mk_comb(`(-):num->num->num`, p64), `2`);;

let failures = ref 0;;

let test name g =
  Printf.printf "TEST %s\n  GOAL %s\n%!" name (string_of_term g);
  let ok =
    try ignore(prove(g, CONV_TAC BRIDGE_TO_WORD_CONV THEN CSAT_TAC)); true
    with Failure s ->
      Printf.printf "  reason %s\n%!" s; false
       | e -> Printf.printf "  reason %s\n%!" (Printexc.to_string e); false in
  if ok then Printf.printf "BRIDGE_FLAG_REGRESSION %s PASS\n%!" name
  else (incr failures;
        Printf.printf "BRIDGE_FLAG_REGRESSION %s FAIL (equation was not encoded)\n%!"
          name);;

(* c16 = 0 forces s16 in {0,1}; c16 = 1 forces s16 in {2^64-2, 2^64-1}. *)
test "split_zero" (goal `0` (le (val64 v_s16) `1`));;
test "split_one" (goal `1` (le two64m2 (val64 v_s16)));;

(* and the unsplit form, where c16 is still a variable *)
let unsplit =
  mk_imp (mk_eq (iadd (imul (ineg (ipow2 `64`)) (ib (bit `(c16:bool)`)))
                      (ib (val64 v_s16)),
                isub (isub (ib (bit v_c8)) (ib `0`)) (ib (bit v_c15))),
          num_or (le (val64 v_s16) `1`) (le two64m2 (val64 v_s16)));;
test "unsplit_bitval" unsplit;;

Printf.printf "BRIDGE_FLAG_REGRESSION_SUMMARY failures=%d\n%!" !failures;;
if !failures > 0 then exit 1;;
