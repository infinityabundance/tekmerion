#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;

let p : term = `(&2:int) pow 64 * &(val (m1:int64))`;;
Printf.printf "P %s : %s\n" (string_of_term p) (string_of_type (type_of p));;
let (mul,pow64,valm1) = (rator(rator p), rator p, rand p);;
let (powop,two,n) = (rator(rator pow64), rand(rator pow64), rand pow64);;
Printf.printf "MULOP %s\n" (fst(dest_const mul));;
Printf.printf "POWOP %s\n" (fst(dest_const powop));;
Printf.printf "TWO %s aconv=%b\n" (string_of_term two) (aconv two `(&2):int`);;
Printf.printf "N %s is_num=%b\n" (string_of_term n) (is_numeral n);;
Printf.printf "Q %s\n" (string_of_term valm1);;
Printf.printf "DESTQ %s\n" (fst(dest_const (rator valm1)));;
(try Printf.printf "CONV => %s\n%!" (string_of_term (rhs(concl(INT_SHIFT_TO_JOIN_CONV p))))
 with e -> Printf.printf "CONV !! %s\n%!" (Printexc.to_string e));;
(try Printf.printf "CONV2 => %s\n%!" (string_of_term (rhs(concl(BRIDGE_SHIFTS_TO_JOINS_CONV p))))
 with e -> Printf.printf "CONV2 !! %s\n%!" (Printexc.to_string e));;
Printf.printf "TEST6_DONE\n%!";;
