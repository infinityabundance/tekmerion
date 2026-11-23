#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;

let tryjoin name tm k =
  try
    let th = INT_JOIN_ZERO_TH tm k in
    Printf.printf "%s OK => %s\n%!" name (string_of_term (concl th))
  with e -> Printf.printf "%s !! %s\n%!" name (Printexc.to_string e);;

tryjoin "join64+64" `(m1:int64)` 64;;
tryjoin "join128+64" `((word_join (m1:int64) (m0:int64)):int128)` 64;;
tryjoin "join192+64" `((word_join (m2:int64) ((word_join (m1:int64) (m0:int64)):int128)):int192)` 64;;
Printf.printf "DIM64 %s\n" (string_of_term (rhs (concl (DIMINDEX_CONV `dimindex(:64)`))));;
Printf.printf "DIM128 %s\n" (string_of_term (rhs (concl (DIMINDEX_CONV `dimindex(:128)`))));;
Printf.printf "DIM192 %s\n" (string_of_term (rhs (concl (DIMINDEX_CONV `dimindex(:192)`))));;
Printf.printf "DIM256 %s\n" (string_of_term (rhs (concl (DIMINDEX_CONV `dimindex(:256)`))));;
Printf.printf "TEST4_DONE\n%!";;
