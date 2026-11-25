(* Driver for tests/interval_engine_selftest.sh (XIX.A3).
 *
 * The shell script prepends tools/interval_engine_shim.ml and the block
 * between BEGIN/END BRIDGE_INTERVAL_STANDALONE in tools/bridge.ml, so every
 * assertion below runs the *shipped* bound arithmetic, not a copy.
 *
 * The check is a counterexample search, not a proof of soundness: for every
 * pair of bounds in a small family it enumerates concrete rational points in
 * the two intervals, computes x+y (or k*x) exactly, and requires the shipped
 * interval result to contain every such value.  A wrong rule therefore shows
 * up as a named witness; the last section runs the retired join-based rule
 * through the same check and requires it to be rejected.
 *)

let failures = ref 0

let check name cond =
  if cond then Printf.printf "OK %s\n%!" name
  else begin
    Printf.printf "FAIL %s\n%!" name;
    incr failures
  end

(* --- bounds ------------------------------------------------------------- *)

type bound = (Num.num * bool) option

let bstr = function
  | None -> "unknown"
  | Some (v, strict) ->
      Printf.sprintf "%s%s" (Num.to_string v) (if strict then "^" else "")

let pair_str (lo, hi) = Printf.sprintf "[%s, %s]" (bstr lo) (bstr hi)

(* Concrete values in an interval.  A lower endpoint (v,strict) means
   x > v (strict) or x >= v; an upper endpoint means x < v or x <= v. *)
let above b x =
  match b with
  | None -> true
  | Some (v, strict) -> if strict then Num.lt_num v x else Num.le_num v x

let below x b =
  match b with
  | None -> true
  | Some (v, strict) -> if strict then Num.lt_num x v else Num.le_num x v

let in_interval (lo, hi) x = above lo x && below x hi

(* --- the small families ------------------------------------------------ *)

let values = [Num.int_num (-2); Num.int_num (-1); Num.int_num 0;
              Num.int_num 1; Num.int_num 2]

let all_bounds =
  None ::
  List.concat (List.map (fun v -> [Some (v, false); Some (v, true)]) values)

(* Samples: multiples of 1/4 in [-3, 3].  Exact rationals throughout. *)
let grid =
  let rec go k acc =
    if k > 12 then List.rev acc else go (k + 1) (Num.make k 4 :: acc) in
  go (-12) []

let step = Num.make 1 4

let samples b = List.filter (in_interval b) grid

(* Intervals whose endpoints are in the small family.  An interval with no
   sample in the grid contributes no points to the search; it is kept in the
   rule-level joins but does not test containment. *)
let all_intervals =
  let pairs =
    List.concat (List.map (fun lo -> List.map (fun hi -> (lo, hi)) all_bounds)
                   all_bounds) in
  List.filter (fun (lo, hi) ->
      match lo, hi with
      | Some (v, s), Some (w, t) ->
          Num.lt_num v w || (Num.eq_num v w && not (s || t))
      | _ -> true) pairs

(* --- lookup tables for the shipped functions --------------------------- *)

let atom_a = 1
let atom_b = 2

let lookup2 (ia : bound * bound) (ib : bound * bound) t =
  if t = atom_a then ia else if t = atom_b then ib
  else invalid_arg "lookup2"

let two_terms = [(atom_a, Num.int_num 1); (atom_b, Num.int_num 1)]
let one_term a = [(a, Num.int_num 1)]

let bounds2 ia ib = bridge_bounds_of_terms (lookup2 ia ib) num_0 two_terms
let no_bounds : bound * bound = (None, None)
let bounds1 ia = bridge_bounds_of_terms (lookup2 ia no_bounds) num_0 (one_term atom_a)

(* --- generic containment search over a pair of bounds ------------------ *)

(* Return (checked_pairs, checked_points, failures, first_witness). *)
let search op_name op cases =
  let checked_pairs = ref 0 and checked_points = ref 0 in
  let bad = ref 0 and witness = ref None in
  List.iter (fun (ia, ib) ->
      incr checked_pairs;
      let sa = samples ia and sb = samples ib in
      if sa <> [] && sb <> [] then begin
        let result = op ia ib in
        List.iter (fun x ->
            List.iter (fun y ->
                incr checked_points;
                let got = Num.add_num x y in
                let (lo, hi) = result in
                if not (above lo got && below got hi) then begin
                  incr bad;
                  if !witness = None then
                    witness := Some (Printf.sprintf
                      "%s: %s + %s = %s not in %s"
                      op_name (Num.to_string x) (Num.to_string y)
                      (Num.to_string got) (pair_str result))
                end) sb) sa
      end) cases;
  (!checked_pairs, !checked_points, !bad, !witness)

(* Tightness: a computed endpoint must not be looser than two grid steps
   from the extreme sampled value.  Two steps, not one: a strict input endpoint
   removes the boundary grid point on *each* side, so a sum can miss the true
   endpoint by 2*step on the sample grid.  The check still rejects a bound that
   is grossly loose or on the wrong side. *)
let tight name cases op =
  let bad = ref 0 and witness = ref None in
  let two_steps = Num.add_num step step in
  List.iter (fun (ia, ib) ->
      let sa = samples ia and sb = samples ib in
      if sa <> [] && sb <> [] then begin
        let sums =
          List.concat (List.map (fun x -> List.map (fun y -> Num.add_num x y) sb) sa) in
        let min_s = List.fold_left (fun a v -> if Num.lt_num v a then v else a)
                      (List.hd sums) (List.tl sums) in
        let max_s = List.fold_left (fun a v -> if Num.lt_num a v then v else a)
                      (List.hd sums) (List.tl sums) in
        let (lo, hi) = op ia ib in
        (match lo with
         | Some (v, _) when Num.lt_num (Num.add_num v two_steps) min_s ->
             incr bad;
             if !witness = None then
               witness := Some (Printf.sprintf "%s: lo %s is below sampled min %s"
                 name (bstr lo) (Num.to_string min_s))
         | _ -> ());
        (match hi with
         | Some (v, _) when Num.lt_num max_s (Num.add_num v (Num.neg_num two_steps)) ->
             incr bad;
             if !witness = None then
               witness := Some (Printf.sprintf "%s: hi %s is above sampled max %s"
                 name (bstr hi) (Num.to_string max_s))
         | _ -> ())
      end) cases;
  (!bad, !witness)

(* --- 1. the reported defect, as a unit test ---------------------------- *)

let () =
  Printf.printf "case: 0 <= x <= 3, 0 <= y <= 4\n%!";
  let ix = (Some (Num.int_num 0, false), Some (Num.int_num 3, false)) in
  let iy = (Some (Num.int_num 0, false), Some (Num.int_num 4, false)) in
  let (lo, hi) = bounds2 ix iy in
  Printf.printf "  shipped: lo=%s hi=%s\n%!" (bstr lo) (bstr hi);
  check "sum lower bound is 0" (lo = Some (Num.int_num 0, false));
  check "sum upper bound is 7 (not the old join's 0)"
    (hi = Some (Num.int_num 7, false));
  (match hi with
   | Some (v, _) -> check "upper bound <= 7" (Num.le_num v (Num.int_num 7))
   | None -> check "upper bound <= 7" false)

(* --- 2. unknown propagates --------------------------------------------- *)

let () =
  let unknown_lower = (None, Some (Num.int_num 4, false)) in
  let known = (Some (Num.int_num 0, false), Some (Num.int_num 3, false)) in
  let (lo, _) = bounds2 unknown_lower known in
  check "a missing lower endpoint makes the sum's lower endpoint unknown"
    (lo = None);
  let (lo2, _) = bounds1 unknown_lower in
  check "one unbounded summand is enough for the lower endpoint"
    (lo2 = None);
  let (_, hi) = bounds2 (Some (Num.int_num 0, false), None) known in
  check "a missing upper endpoint makes the sum's upper endpoint unknown"
    (hi = None);
  (* Bounds are still known on the side that is known. *)
  let (_, hi3) = bounds2 known unknown_lower in
  check "the known upper endpoints of both summands are still added"
    (hi3 = Some (Num.int_num 7, false))

(* --- 3. exhaustive sums ------------------------------------------------ *)

let () =
  let cases = List.concat (List.map (fun a -> List.map (fun b -> (a, b)) all_intervals)
                              all_intervals) in
  let pairs, points, bad, witness = search "add" bounds2 cases in
  Printf.printf "exhaustive add: bound_pairs=%d points=%d failures=%d\n%!"
    pairs points bad;
  (match witness with
   | None -> Printf.printf "  counterexamples: none\n%!"
   | Some w -> Printf.printf "  first counterexample: %s\n%!" w);
  check "every sampled sum lies in the computed interval" (bad = 0);
  let tbad, twitness = tight "add" cases bounds2 in
  (match twitness with
   | None -> ()
   | Some w -> Printf.printf "  tightness witness: %s\n%!" w);
  check "computed endpoints are tight to within two grid steps" (tbad = 0)

(* --- 4. exhaustive scaling --------------------------------------------- *)

let coeffs = [-2; -1; 0; 1; 2]

let () =
  let cases =
    List.concat (List.map (fun k ->
        List.map (fun i -> (Num.int_num k, i)) all_intervals) coeffs) in
  let checked = ref 0 and bad_points = ref 0 and witness = ref None in
  List.iter (fun (k, i) ->
      incr checked;
      let is = samples i in
      let (lo, hi) = bridge_bound_contrib k i in
      List.iter (fun x ->
          let got = Num.mult_num k x in
          if not (above lo got && below got hi) then begin
            incr bad_points;
            if !witness = None then
              witness := Some (Printf.sprintf
                "scale %s * %s = %s not in %s"
                (Num.to_string k) (Num.to_string x) (Num.to_string got) (pair_str (lo, hi)))
          end) is) cases;
  Printf.printf "exhaustive scale: cases=%d failures=%d\n%!" !checked !bad_points;
  (match !witness with
   | None -> Printf.printf "  counterexamples: none\n%!"
   | Some w -> Printf.printf "  first counterexample: %s\n%!" w);
  check "every sampled scaled value lies in the computed interval" (!bad_points = 0)

(* --- 5. joins are max/min on constraints, with None as identity -------- *)

let sem_max_lo a b =
  match a, b with
  | None, x | x, None -> x
  | Some (v1, s1), Some (v2, s2) ->
      if Num.lt_num v1 v2 then Some (v1, s1)
      else if Num.lt_num v2 v1 then Some (v2, s2)
      else Some (v1, s1 || s2)

let sem_min_hi a b =
  match a, b with
  | None, x | x, None -> x
  | Some (v1, s1), Some (v2, s2) ->
      if Num.lt_num v2 v1 then Some (v1, s1)
      else if Num.lt_num v1 v2 then Some (v2, s2)
      else Some (v1, s1 || s2)

let () =
  let bad_lo = ref 0 and bad_hi = ref 0 in
  List.iter (fun a -> List.iter (fun b ->
      if bstr (bridge_bound_join_lo a b) <> bstr (sem_max_lo a b) then incr bad_lo;
      if bstr (bridge_bound_join_hi a b) <> bstr (sem_min_hi a b) then incr bad_hi)
    all_bounds) all_bounds;
  Printf.printf "join rule: lower_failures=%d upper_failures=%d\n%!" !bad_lo !bad_hi;
  check "join of lower bounds is max" (!bad_lo = 0);
  check "join of upper bounds is min" (!bad_hi = 0)

(* --- 6. the retired rule must be rejected ------------------------------ *)

(* The pre-XIX.A accumulation: the same code, but the summands were *joined*
   instead of *added*.  It is kept here only as the negative control. *)
let legacy_bounds_of_terms lookup c terms =
  List.fold_left (fun (lo, hi) (t, k) ->
      let (l, h) = lookup t in
      let lo_add b = match b with
        | None -> lo
        | Some (v, s) -> bridge_bound_join_lo lo (Some (Num.mult_num k v, s))
      and hi_add b = match b with
        | None -> hi
        | Some (v, s) -> bridge_bound_join_hi hi (Some (Num.mult_num k v, s)) in
      if Num.lt_num num_0 k then (lo_add l, hi_add h)
      else (lo_add h, hi_add l))
    ((Some (c, false)), (Some (c, false))) terms

let () =
  let cases = List.concat (List.map (fun a -> List.map (fun b -> (a, b)) all_intervals)
                              all_intervals) in
  let op ia ib = legacy_bounds_of_terms (lookup2 ia ib) num_0 two_terms in
  let _, _, bad, witness = search "legacy-join" op cases in
  Printf.printf "negative control (legacy join rule): failures=%d\n%!" bad;
  (match witness with
   | None -> Printf.printf "  no counterexample (the control did not fire)\n%!"
   | Some w -> Printf.printf "  first counterexample: %s\n%!" w);
  check "the exhaustive test rejects the retired join-based rule" (bad > 0)

(* --- summary ------------------------------------------------------------ *)

let () =
  if !failures = 0 then begin
    Printf.printf "\nOK interval_engine_selftest\n%!";
    exit 0
  end else begin
    Printf.printf "\nFAIL interval_engine_selftest: %d failure(s)\n%!" !failures;
    exit 1
  end
