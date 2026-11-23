(* Fresh-process check of tools/bridge.ml plus the new derivation lemmas. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
let ax0 = List.length (axioms ());;
let bt0 = Unix.gettimeofday ();;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;
Printf.printf "AXIOMS before=%d after=%d\n" ax0 (List.length (axioms ()));;
let _ = map (fun (nm,th) -> Printf.printf "HAVE %s hyps=%d\n" nm (List.length (hyp th))) [
 ("EXP64_SUC",EXP64_SUC); ("CARRY_CHAIN_ADD_STEP",CARRY_CHAIN_ADD_STEP);
 ("CARRY_CHAIN_ADD_BASE",CARRY_CHAIN_ADD_BASE);
 ("CARRY_CHAIN_SUB_STEP",CARRY_CHAIN_SUB_STEP);
 ("CARRY_CHAIN_SUB_BASE",CARRY_CHAIN_SUB_BASE);
 ("SUB_FLAG_ARITH",SUB_FLAG_ARITH); ("SUB_CHAIN_FLAG",SUB_CHAIN_FLAG);
 ("BIGNUM_BOUND",BIGNUM_BOUND); ("FLAG_LE",FLAG_LE);
 ("ADD_ACC_NUM_0",ADD_ACC_NUM_0); ("ADD_ACC_NUM",ADD_ACC_NUM);
 ("SBC_ACC_NUM_0",SBC_ACC_NUM_0); ("SBC_ACC_NUM",SBC_ACC_NUM);
 ("ADC_NUM",ADC_NUM); ("BRIDGE_REAL_BITVAL_NOT",BRIDGE_REAL_BITVAL_NOT);
 ("REAL_EXP64_SUB1",REAL_EXP64_SUB1); ("VAL_WORD_ALLONES",VAL_WORD_ALLONES);
 ("ADC_ALLONES_NUM",ADC_ALLONES_NUM);
 ("VAL_WORD_ADD_BITVAL",VAL_WORD_ADD_BITVAL);
 ("BITVAL_ADD_ZERO",BITVAL_ADD_ZERO); ("CSETM_EQ",CSETM_EQ)];;
Printf.printf "BRIDGE_CHECK_DONE seconds=%.3f\n%!" (Unix.gettimeofday () -. bt0);;
