#load "unix.cma";;
needs "arm/proofs/base.ml";;
let t name tm =
  try let th = prove(tm, ARITH_TAC) in
      Printf.printf "%s OK\n%!" name
  with e -> Printf.printf "%s FAIL %s\n%!" name (Printexc.to_string e);;
t "g1" `!n a b:num. b < 2 EXP n /\ 2 EXP n <= a + b ==> a + b - 2 EXP n < a`;;
t "g2" `!n a b:num. 2 EXP n <= a + b ==> (a + b - 2 EXP n) + 2 EXP n = a + b`;;
t "g3" `!n a b:num. a + b < 2 EXP n ==> ~(a + b < a)`;;
t "g4" `!(x:W word) (y:W word). val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\ val x + val y < 2 EXP dimindex(:W) ==> ~(val x + val y < val x)`;;
t "g5" `!(x:W word) (y:W word). val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\ ~(val x + val y < 2 EXP dimindex(:W)) ==> val x + val y - 2 EXP dimindex(:W) < val x`;;
t "g6" `!(x:W word) (y:W word). val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\ ~(val x + val y < 2 EXP dimindex(:W)) ==> (val x + val y - 2 EXP dimindex(:W)) + 2 EXP dimindex(:W) = val x + val y`;;
Printf.printf "ARITH_TEST_DONE\n%!";;
