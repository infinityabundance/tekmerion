#load "unix.cma";;
needs "arm/proofs/base.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;
Printf.printf "ADD %s\n" (string_of_thm INT_OF_NUM_ADD);;
Printf.printf "MUL %s\n" (string_of_thm INT_OF_NUM_MUL);;
Printf.printf "POW %s\n" (string_of_thm INT_OF_NUM_POW);;
let t = `&(2 EXP 64 * val (m:int64) + val (n:int64))`;;
let show tag conv tm =
  (try Printf.printf "%s => %s\n%!" tag (string_of_term (rhs (concl (conv tm))))
   with e -> Printf.printf "%s !! %s\n%!" tag (Printexc.to_string e));;
show "RW" (REWRITE_CONV[INT_OF_NUM_ADD; INT_OF_NUM_MUL; INT_OF_NUM_POW]) t;;
show "GEN_TOPDEPTH" (GEN_REWRITE_CONV TOP_DEPTH_CONV [INT_OF_NUM_ADD; INT_OF_NUM_MUL; INT_OF_NUM_POW]) t;;
show "GEN_DEPTH" (GEN_REWRITE_CONV DEPTH_CONV [INT_OF_NUM_ADD; INT_OF_NUM_MUL; INT_OF_NUM_POW]) t;;
show "GEN_I" (GEN_REWRITE_CONV I [INT_OF_NUM_ADD; INT_OF_NUM_MUL; INT_OF_NUM_POW]) t;;
Printf.printf "TYPES t=%s arg=%s\n" (string_of_type (type_of t)) (string_of_type (type_of (rand t)));;
Printf.printf "TEST2_DONE\n%!";;
