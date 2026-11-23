(* Fresh-process receipt for the reference-route lemmas (V8.18 / r4).
   Loads the shipped bridge in a new process, prints every new lemma's statement
   and the axiom count before/after.  Nothing is transcribed from a human proof
   here; the statements are the ones in tools/bridge.ml. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
let ax0 = List.length (axioms ()) in
Printf.printf "AXIOMS before=%d\n%!" ax0;;
let t0 = ref (Unix.gettimeofday ());;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;
Printf.printf "AXIOMS after=%d\n%!" (List.length (axioms ()));;
let show name th = Printf.printf "REF_LEMMA %s : %s\n%!" name
  (string_of_term (concl th));;
show "MOD_ADD_SELF" MOD_ADD_SELF;;
show "SUB_REARRANGE" SUB_REARRANGE;;
show "REF_SUB_KEEP" REF_SUB_KEEP;;
show "REF_SUB_BORROW" REF_SUB_BORROW;;
show "ref_sub" ref_sub;;
show "SUB_REF_VAL" SUB_REF_VAL;;
show "SUB_REF_VAL_WORD" SUB_REF_VAL_WORD;;
show "ref_neg" ref_neg;;
show "NEG_REF_VAL" NEG_REF_VAL;;
show "NEG_REF_VAL_WORD" NEG_REF_VAL_WORD;;
show "SUB_INT_REM_MOD" SUB_INT_REM_MOD;;
Printf.printf "REF_HYPS ref_sub=%d SUB_REF_VAL=%d SUB_REF_VAL_WORD=%d NEG_REF_VAL=%d NEG_REF_VAL_WORD=%d SUB_INT_REM_MOD=%d\n%!"
  (List.length (hyp ref_sub)) (List.length (hyp SUB_REF_VAL))
  (List.length (hyp SUB_REF_VAL_WORD)) (List.length (hyp NEG_REF_VAL))
  (List.length (hyp NEG_REF_VAL_WORD)) (List.length (hyp SUB_INT_REM_MOD));;
Printf.printf "REF_CHECK_DONE seconds=%.3f\n%!" (Unix.gettimeofday () -. !t0);;
