(* Fresh-process verification of every binding in tools/bridge.ml.
   The loaded file is the mechanically instrumented copy; see instrument.py.
   csat_lib.ml is loaded first because BRIDGE_CSAT_TAC calls CSAT_TAC. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
let ax0 = List.length (axioms ());;
let bt0 = Unix.gettimeofday ();;
loadt "/mnt/1tb_kingston/Tekmerion/research/receipts/bridge/bridge_instrumented.ml";;
Printf.printf "AXIOMS before=%d after=%d\n" ax0 (List.length (axioms ()));;
Printf.printf "BRIDGE_CHECK_DONE seconds=%.3f\n%!" (Unix.gettimeofday () -. bt0);;
