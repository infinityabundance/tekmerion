(* Synthetic stand-in for .bench/hol-light/Cadical/{dimacs,cnf,ldrup}.ml in the
 * offline IX.66 marker test. Those three files are inlined but never
 * instrumented, so only their position in the inline order matters here; the
 * real files need HOL Light and are read only by the half of the test that
 * runs when `.bench` is present.
 *)

let dimacs_of_hol = fun cnfname tm -> ignore (cnfname, tm);;

let read_dimacs = fun path -> ignore path; [];

let clauses_of_term = fun tm -> ignore tm; [];
