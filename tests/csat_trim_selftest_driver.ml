(* Driver for tests/csat_trim_selftest.sh.
 *
 * The shell script prepends the text between BEGIN/END CSAT_TRIM_STANDALONE in
 * tools/csat_lib.ml, so these assertions exercise the shipped trimmer verbatim
 * outside HOL Light. Every case is a temp file in the current directory; the
 * trimmer rewrites the file in place and prints one CSAT_TRIM line. *)

let read_file path =
  let ic = open_in_bin path in
  let n = in_channel_length ic in
  let s = really_input_string ic n in
  close_in ic;
  s

let write_file path s =
  let oc = open_out_bin path in
  output_string oc s;
  close_out oc

let failures = ref 0

let check name cond =
  if cond then Printf.printf "OK %s\n%!" name
  else begin
    Printf.printf "FAIL %s\n%!" name;
    incr failures
  end

(* Run the trimmer and compare the resulting file with [expected]. *)
let run name input expected =
  write_file "case.lrat" input;
  csat_trim_lrat "case.lrat";
  let got = read_file "case.lrat" in
  check name (got = expected)

(* The refusal paths must leave the bytes untouched. *)
let run_unchanged name input =
  write_file "case.lrat" input;
  csat_trim_lrat "case.lrat";
  let got = read_file "case.lrat" in
  check name (got = input)

let () =
  (* 1. The backward closure keeps the chain and drops the unused addition. *)
  run "chain_drops_unused"
    "1 1 0 0\n2 2 0 0\n9 -1 0 1 0\n3 3 0 0\n4 1 2 0 1 9 0\n7 0 1 2 4 0\n"
    "1 1 0 0\n2 2 0 0\n9 -1 0 1 0\n4 1 2 0 1 9 0\n7 0 1 2 4 0\n";

  (* 2. Every deletion line is dropped, including one of a kept clause. *)
  run "deletions_dropped"
    "10 1 0 0\n11 2 0 0\n10 d 11 0\n12 0 10 11 0\n"
    "10 1 0 0\n11 2 0 0\n12 0 10 11 0\n";

  (* 3. Kept lines stay in the original file order. *)
  run "original_order"
    "5 -2 0 7 0\n7 2 0 0\n1 1 0 0\n8 0 5 0\n"
    "5 -2 0 7 0\n7 2 0 0\n8 0 5 0\n";

  (* 4. A trailing deletion is not the root; the last addition is. *)
  run "trailing_deletion"
    "1 1 0 0\n2 0 1 0\n3 d 1 0\n"
    "1 1 0 0\n2 0 1 0\n";

  (* 5. RAT guard: any negative hint refuses the trim, bytes untouched. *)
  run_unchanged "negative_hint_skips"
    "1 1 0 0\n2 2 0 0\n3 0 -1 0\n";

  (* 6. No empty clause in the root refuses the trim, bytes untouched. *)
  run_unchanged "no_empty_clause_skips"
    "1 1 0 0\n2 2 0 0\n4 1 2 0 1 2 0\n";

  (* 7. Malformed lines refuse rather than silently dropping data. *)
  run_unchanged "malformed_skips"
    "1 1 0 0\nnot lrat\n2 0 1 0\n";
  run_unchanged "empty_file_skips" "";
  run_unchanged "deletion_only_skips" "1 d 1 0\n";

  (* 8. Reused addition ids are ambiguous, so the trim is refused. *)
  run_unchanged "duplicate_ids_skips"
    "1 1 0 0\n1 2 0 0\n2 0 1 0\n";

  (* 9. A hint to a CNF clause is not an addition and is not kept. *)
  run "cnf_clause_hint_is_not_addition"
    "100 -3 0 1 2 0\n101 0 100 0\n"
    "100 -3 0 1 2 0\n101 0 100 0\n";

  if !failures = 0 then Printf.printf "PASS csat_trim_selftest cases=11\n%!"
  else (Printf.printf "FAIL csat_trim_selftest failures=%d\n%!" !failures;
        exit 1)
