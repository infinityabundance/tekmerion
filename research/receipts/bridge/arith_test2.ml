#load "unix.cma";;
needs "arm/proofs/base.ml";;
let t name tm =
  try let _ = prove(tm, ARITH_TAC) in Printf.printf "%s OK\n%!" name
  with e -> Printf.printf "%s FAIL\n%!" name;;
t "h1" `!n a b:num. 0 < 2 EXP n /\ b < 2 EXP n /\ 2 EXP n <= a + b ==> a + b - 2 EXP n < a`;;
t "h2" `!n a b:num. 0 < 2 EXP n /\ 2 EXP n <= a + b ==> (a + b - 2 EXP n) + 2 EXP n = a + b`;;
t "h3" `!(x:W word) (y:W word). 0 < 2 EXP dimindex(:W) /\ val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\ ~(val x + val y < 2 EXP dimindex(:W)) ==> val x + val y - 2 EXP dimindex(:W) < val x`;;
t "h4" `!(x:W word) (y:W word). 0 < 2 EXP dimindex(:W) /\ val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\ ~(val x + val y < 2 EXP dimindex(:W)) ==> (val x + val y - 2 EXP dimindex(:W)) + 2 EXP dimindex(:W) = val x + val y`;;
Printf.printf "ARITH_TEST2_DONE\n%!";;
