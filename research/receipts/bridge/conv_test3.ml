#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;
let t =
 `-- &(2 EXP 64) * &(bitval (b:bool)) + &(val (s16:int64)) =
  &(val (s3:int64)) + &(val (n0:int64))`;;
Printf.printf "TYPE %s\n" (string_of_type (type_of t));;
let show tag conv tm =
  (try Printf.printf "%s => %s\n%!" tag (string_of_term (rhs (concl (conv tm))))
   with e -> Printf.printf "%s !! %s\n%!" tag (Printexc.to_string e));;
show "NUM_TO_INT" NUM_TO_INT_CONV t;;
show "FULL" BRIDGE_TO_WORD_CONV t;;
let g = `&(val (m0:int64) + 2 EXP 64 * val (m1:int64) + 2 EXP 128 * val (m2:int64)) =
         &(val (o0:int64) + 2 EXP 64 * val (o1:int64) + 2 EXP 128 * val (o2:int64))`;;
show "GOAL_FULL" BRIDGE_TO_WORD_CONV g;;
Printf.printf "TEST3_DONE\n%!";;
