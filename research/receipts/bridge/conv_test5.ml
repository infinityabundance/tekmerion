#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;

let x = `(m1:int64)`;;
let m = Num.int_of_num (dest_finty (match type_of x with
   | Tyapp("word",[n]) -> n | _ -> failwith "not word"));;
Printf.printf "m=%d type64=%s type128=%s\n" m
  (string_of_type (mk_finty (Num.num_of_int 64)))
  (string_of_type (mk_finty (Num.num_of_int 128)));;
let inst = INST_TYPE
  [mk_finty(Num.num_of_int m), `:M`;
   mk_finty(Num.num_of_int 64), `:N`;
   mk_finty(Num.num_of_int(m + 64)), `:P`] INT_JOIN_ZERO;;
let th1 = SPEC x inst;;
Printf.printf "TH1 %s\n" (string_of_thm th1);;
let cond = lhand(concl th1);;
Printf.printf "COND %s\n" (string_of_term cond);;
let step1 = LAND_CONV(BINOP_CONV DIMINDEX_CONV THENC NUM_ADD_CONV) cond;;
Printf.printf "S1 %s\n" (string_of_term (rhs(concl step1)));;
let step2 = (LAND_CONV(BINOP_CONV DIMINDEX_CONV THENC NUM_ADD_CONV) THENC RAND_CONV DIMINDEX_CONV THENC NUM_EQ_CONV) cond;;
Printf.printf "S2 %s\n" (string_of_term (rhs(concl step2)));;
Printf.printf "TEST5_DONE\n%!";;
