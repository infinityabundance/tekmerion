#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;
let show tag conv tm =
  (try Printf.printf "%s => %s\n%!" tag (string_of_term (rhs (concl (conv tm))))
   with e -> Printf.printf "%s !! %s\n%!" tag (Printexc.to_string e));;
let p : term = `(&2:int) pow 64 * &(val (m1:int64))`;;
show "SHIFT64" INT_SHIFT_TO_JOIN_CONV p;;
let pb : term = `(&2:int) pow 64 * &(bitval (b:bool))`;;
show "SHIFTBIT" INT_SHIFT_TO_JOIN_CONV pb;;
let g = `&(val (m0:int64) + 2 EXP 64 * val (m1:int64) + 2 EXP 128 * val (m2:int64) + 2 EXP 192 * val (m3:int64)) =
         &(val (o0:int64) + 2 EXP 64 * val (o1:int64) + 2 EXP 128 * val (o2:int64) + 2 EXP 192 * val (o3:int64))`;;
show "GOAL4LIMB" BRIDGE_TO_WORD_CONV g;;
let acc = `-- &(2 EXP 64) * &(bitval (b:bool)) + &(val (s16:int64)) =
           &(val (s3:int64)) + &(val (n0:int64))`;;
show "ACC" BRIDGE_TO_WORD_CONV acc;;
let showdim n =
  ignore n;;
Printf.printf "DIM65 %s\n" (string_of_term (rhs(concl (DIMINDEX_CONV `dimindex(:65)`))));;
Printf.printf "DIM129 %s\n" (string_of_term (rhs(concl (DIMINDEX_CONV `dimindex(:129)`))));;
Printf.printf "DIM193 %s\n" (string_of_term (rhs(concl (DIMINDEX_CONV `dimindex(:193)`))));;
Printf.printf "DIM257 %s\n" (string_of_term (rhs(concl (DIMINDEX_CONV `dimindex(:257)`))));;
Printf.printf "TEST7_DONE\n%!";;
