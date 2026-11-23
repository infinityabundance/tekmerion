(* ========================================================================= *)
(* csat_probe.ml -- measure the certified-SAT close on word tautologies.     *)
(* ========================================================================= *)

#load "unix.cma";;

needs "Library/words.ml";;
needs "Cadical/make.ml";;
loadt (Sys.getenv "CSAT_LIB");;

let probe (tag,tm) =
  let t0 = csat_now () in
  let closed,msg =
    try
      let th = prove(tm, CSAT_TAC) in
      (hyp th = [] && concl th = tm, "")
    with Failure s -> (false, s)
       | _ -> (false, "unknown exception") in
  let dt = csat_now () -. t0 in
  if closed then
    Printf.printf "CSAT_RESULT %s closed=1 seconds=%.3f\n" tag dt
  else
    let s = String.concat " " (String.split_on_char '\n' msg) in
    Printf.printf "CSAT_RESULT %s closed=0 seconds=%.3f reason=%s\n" tag dt s;
  flush stdout;;

probe ("word64_carry",
  `!x:int64 y:int64.
     val(word_add x y) = val x + val y \/
     val(word_add x y) + 2 EXP 64 = val x + val y`);;
probe ("word128_add_comm",
  `!x:128 word y:128 word. word_add x y = word_add y x`);;
probe ("word256_add_comm",
  `!x:256 word y:256 word. word_add x y = word_add y x`);;
probe ("word256_carry",
  `!x:256 word y:256 word.
     val(word_add x y) = val x + val y \/
     val(word_add x y) + 2 EXP 256 = val x + val y`);;
probe ("word320_carry",
  `!x:320 word y:320 word.
     val(word_add x y) = val x + val y \/
     val(word_add x y) + 2 EXP 320 = val x + val y`);;

Printf.printf "CSAT_PROBE_DONE\n";;
flush stdout;;
