(* Isolate the conversion behaviour on the exact int shapes in the residual. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;

let t1 = `&(2 EXP 64 * val (m:int64) + val (n:int64)) = &(val (s:int64)) + &1`;;
let show tag conv tm =
  (try Printf.printf "%s => %s\n%!" tag (string_of_term (rhs (concl (conv tm))))
   with e -> Printf.printf "%s !! %s\n%!" tag (Printexc.to_string e));;

show "REAL_TO_INT" REAL_TO_INT_CONV t1;;
show "NUM_TO_INT" NUM_TO_INT_CONV t1;;
show "DISTRIB" INT_DISTRIB_CONV (rhs (concl (NUM_TO_INT_CONV t1)));;
show "SHIFTS" BRIDGE_SHIFTS_TO_JOINS_CONV (rhs (concl (NUM_TO_INT_CONV t1)));;
show "FULL" BRIDGE_TO_WORD_CONV t1;;

let t2 = `-- &(2 EXP 64) * &(bitval b) + &(val (s:int64)) = &(val (s8:int64)) - &1`;;
show "T2 NUM_TO_INT" NUM_TO_INT_CONV t2;;
show "T2 FULL" BRIDGE_TO_WORD_CONV t2;;
Printf.printf "CONV_TEST_DONE\n%!";;
