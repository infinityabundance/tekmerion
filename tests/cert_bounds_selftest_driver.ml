(* Driver for tests/cert_bounds_selftest.sh (XXX.A4).
 *
 * The shell script prepends tests/cert_bounds_shim.ml and the block between
 * BEGIN/END BRIDGE_CERT_STANDALONE in tools/bridge.ml, so every assertion
 * below runs the *shipped* numeric arithmetic:
 *
 *   bridge_cert_bound_core  -- rational division -> exact integer endpoint
 *   bridge_cert_join_lo/hi  -- max/min of lower/upper constraints
 *   bridge_cert_hi/lo       -- recursive bound, incl. strict tightening
 *
 * The checks are semantic, not copies of the rules:
 *
 *   - bound_core: enumerate every integer x in a window, keep the points that
 *     satisfy `k*x + c >= 0' (or `> 0'), and require the emitted endpoint to be
 *     exactly the minimum (k>0) or maximum (k<0) feasible x.
 *   - joins: require the joined constraint to describe exactly the intersection
 *     of the two inputs on a rational sample grid.
 *   - hi/lo: enumerate feasible integer points and require the returned bound
 *     to be the true max/min of the target (for the single-relation cases), or
 *     at least to dominate it when a rational intermediate forbids tightening.
 *
 * Negative controls reproduce the pre-fix rules and require this harness to
 * reject them, so a regression cannot pass silently.
 *)

let failures = ref 0
let checks = ref 0

let check name cond =
  incr checks;
  if cond then Printf.printf "OK %s\n%!" name
  else begin
    Printf.printf "FAIL %s\n%!" name;
    incr failures
  end

let i = Num.int_num
let q n d = Num.div_num (i n) (i d)
let str = Num.to_string

(* --- forms: (atom * coefficient) list * constant ------------------------ *)

let eval (xs,c) env =
  List.fold_left (fun acc (a,k) ->
      Num.add_num acc (Num.mult_num k (i (List.nth env a)))) c xs

let satisfies (xs,c) strict env =
  let v = eval (xs,c) env in
  if strict then Num.lt_num (i 0) v else Num.le_num (i 0) v

let window lo hi = List.init (hi - lo + 1) (fun j -> lo + j)

(* single-variable problems: fill both possible atom slots with the sample so
   a form on atom 0 or atom 1 is evaluated consistently. *)
let feasible1 f strict =
  List.filter_map (fun x ->
      let env = [x; x] in
      if satisfies f strict env then Some env else None)
    (window (-40) 40)

(* `better v m' is true when the new value `v' is better than `m'. *)
let extremum better target pts =
  match pts with
  | [] -> failwith "no feasible integer points"
  | x0 :: rest ->
      List.fold_left (fun m env ->
          let v = eval target env in
          if better v m then v else m) (eval target x0) rest

let true_max1 target f strict =
  extremum (fun v m -> Num.lt_num m v) target (feasible1 f strict)

let true_min1 target f strict =
  extremum (fun v m -> Num.lt_num v m) target (feasible1 f strict)

(* --- 0. the five endpoint rules, stated explicitly ---------------------- *)

let () =
  Printf.printf
    "rules: (1) x<q => x<=ceil(q)-1  (2) x<=q => x<=floor(q)\n\
     \       (3) x>q => x>=floor(q)+1  (4) x>=q => x>=ceil(q)\n\
     \       (5) strict/non-strict tightening under a rational multiplier\n%!"

(* --- 1. bridge_cert_bound_core over the small grid ---------------------- *)

let chk_lo name k c strict e =
  check name
    (match bridge_cert_bound_core (i k) (i c) strict with
     | Some(v,s),None -> (not s) && Num.eq_num v (i e)
     | _ -> false)

let chk_hi name k c strict e =
  check name
    (match bridge_cert_bound_core (i k) (i c) strict with
     | None,Some(v,s) -> (not s) && Num.eq_num v (i e)
     | _ -> false)

let () =
  check "k=0 emits no bound"
    (bridge_cert_bound_core (i 0) (i 7) true = (None,None));
  (* rule 4: k>0, non-strict *)
  chk_lo "rule 4: 2x+1 >= 0  -> x >= 0" 2 1 false 0;
  chk_lo "rule 4: 2x-1 >= 0  -> x >= 1" 2 (-1) false 1;
  (* rule 3: k>0, strict *)
  chk_lo "rule 3: 2x+1 > 0   -> x >= 0" 2 1 true 0;
  chk_lo "rule 3: 2x-1 > 0   -> x >= 1" 2 (-1) true 1;
  (* rule 2: k<0, non-strict *)
  chk_hi "rule 2: -2x+1 >= 0 -> x <= 0" (-2) 1 false 0;
  chk_hi "rule 2: -2x-1 >= 0 -> x <= -1" (-2) (-1) false (-1);
  (* rule 1: k<0, strict *)
  chk_hi "rule 1: -2x+1 > 0  -> x <= 0" (-2) 1 true 0;
  chk_hi "rule 1: -2x-1 > 0  -> x <= -1" (-2) (-1) true (-1)

let () =
  let total = ref 0 and bad = ref 0 and first = ref None in
  for k = -4 to 4 do
    if k <> 0 then
    for c = -8 to 8 do
      List.iter (fun strict ->
          incr total;
          let (lo,hi) = bridge_cert_bound_core (i k) (i c) strict in
          let feasible =
            List.filter (fun x ->
                let v = i (k * x + c) in
                if strict then Num.lt_num (i 0) v else Num.le_num (i 0) v)
              (window (-40) 40) in
          let ok =
            if k > 0 then
              match lo,hi with
              | Some(v,s),None ->
                  (not s) && Num.is_integer_num v &&
                  (match feasible with
                   | [] -> false
                   | m::_ -> Num.eq_num v (i m))
              | _ -> false
            else
              match lo,hi with
              | None,Some(v,s) ->
                  (not s) && Num.is_integer_num v &&
                  (match List.rev feasible with
                   | [] -> false
                   | m::_ -> Num.eq_num v (i m))
              | _ -> false in
          if not ok then begin
            incr bad;
            if !first = None then first := Some (k,c,strict)
          end)
        [false; true]
    done
  done;
  (match !first with
   | None -> Printf.printf "  bound_core: no counterexample\n%!"
   | Some (k,c,strict) ->
       Printf.printf "  first bound_core counterexample: k=%d c=%d strict=%b\n%!"
         k c strict);
  Printf.printf "bound_core grid: k in [-4,4]\\{0} x c in [-8,8] x strict: cases=%d failures=%d\n%!"
    !total !bad;
  check "bound_core is the exact integer endpoint on the whole grid"
    (!bad = 0 && !total = 272)

(* --- 2. joins are max/min, and None is the identity --------------------- *)

let vals = [q (-3) 2; i (-1); q (-1) 2; i 0; q 1 2; i 1; q 3 2; i 2]

let all_bounds =
  None :: List.concat_map (fun v -> [Some (v,false,()); Some (v,true,())]) vals

let samples = List.init 25 (fun j -> q (j - 12) 4)   (* -3 .. 3, step 1/4 *)

let above b x =
  match b with
  | None -> true
  | Some (v,s,_) -> if s then Num.lt_num v x else Num.le_num v x

let below x b =
  match b with
  | None -> true
  | Some (v,s,_) -> if s then Num.lt_num x v else Num.le_num x v

let exact_join_lo a b = List.for_all (fun x -> above (bridge_cert_join_lo a b) x = (above a x && above b x)) samples
let exact_join_hi a b = List.for_all (fun x -> below x (bridge_cert_join_hi a b) = (below x a && below x b)) samples

let () =
  let bad_lo = ref 0 and bad_hi = ref 0 and pairs = ref 0 in
  List.iter (fun a -> List.iter (fun b ->
      incr pairs;
      if not (exact_join_lo a b) then incr bad_lo;
      if not (exact_join_hi a b) then incr bad_hi)
    all_bounds) all_bounds;
  Printf.printf "join grid: %d x %d pairs=%d lower_failures=%d upper_failures=%d\n%!"
    (List.length all_bounds) (List.length all_bounds) !pairs !bad_lo !bad_hi;
  check "join_lo is the max of lower constraints" (!bad_lo = 0 && !pairs = 289);
  check "join_hi is the min of upper constraints" (!bad_hi = 0)

(* the shipped add path uses both joins *)
let () =
  let tbl1 = bridge_cert_add [] 0 (Some (i 3,false,())) None in
  let tbl2 = bridge_cert_add tbl1 0 (Some (i 5,false,())) None in
  let lo_ok = match List.assoc 0 tbl2 with
    | Some (v,s,_), _ -> (not s) && Num.eq_num v (i 5)
    | _ -> false in
  let tbl3 = bridge_cert_add tbl2 0 None (Some (i 7,false,())) in
  let tbl4 = bridge_cert_add tbl3 0 None (Some (i 4,false,())) in
  let hi_ok = match List.assoc 0 tbl4 with
    | _, Some (v,s,_) -> (not s) && Num.eq_num v (i 4)
    | _ -> false in
  check "cert_add keeps the tighter (max) lower bound" lo_ok;
  check "cert_add keeps the tighter (min) upper bound" hi_ok

(* negative control: the pre-fix join_lo selected the smaller lower bound *)
let legacy_join_lo a b =
  match a,b with
  | None,x | x,None -> x
  | Some(v1,s1,_),Some(v2,s2,_) ->
      if Num.lt_num v1 v2 then a
      else if Num.lt_num v2 v1 then b
      else if s1 then a else b

let () =
  let bad = ref 0 in
  List.iter (fun a -> List.iter (fun b ->
      let sem = List.for_all (fun x ->
          above (legacy_join_lo a b) x = (above a x && above b x)) samples in
      if not sem then incr bad) all_bounds) all_bounds;
  Printf.printf "negative control (legacy join_lo): pairs the semantics rejects=%d\n%!" !bad;
  check "the harness rejects the pre-fix join_lo" (!bad > 0)

(* --- 3. bridge_cert_hi: strict tightening under a rational multiplier --- *)

let target_x = ([ (0, i 1) ], i 0)
let rel coef c = ([ (0, i coef) ], i c)

let run_hi target f strict =
  bridge_cert_hi (=) 3 [] [ (0, "rel", f, strict) ] target

let check_hi_case name target f strict expected =
  match run_hi target f strict with
  | Some (v,_) ->
      let tm = true_max1 target f strict in
      if not (Num.eq_num v (i expected) && Num.eq_num v tm) then
        Printf.printf "  %s: got %s expected %d true_max %s\n%!"
          name (str v) expected (str tm);
      check name (Num.eq_num v (i expected) && Num.eq_num v tm)
  | None -> check name false

let () =
  (* f = -2x + c >=/> 0  <=>  x <=/> c/2; all four half-integer endpoints. *)
  check_hi_case "hi: x < 1/2   -> x <= 0"  target_x (rel (-2) 1) true 0;
  check_hi_case "hi: x <= 1/2  -> x <= 0"  target_x (rel (-2) 1) false 0;
  check_hi_case "hi: x < -1/2  -> x <= -1" target_x (rel (-2) (-1)) true (-1);
  check_hi_case "hi: x <= -1/2 -> x <= -1" target_x (rel (-2) (-1)) false (-1);
  check_hi_case "hi: x < 3/2   -> x <= 1"  target_x (rel (-2) 3) true 1;
  check_hi_case "hi: x <= 3/2  -> x <= 1"  target_x (rel (-2) 3) false 1;
  check_hi_case "hi: x < -3/2  -> x <= -2" target_x (rel (-2) (-3)) true (-2);
  check_hi_case "hi: x <= -3/2 -> x <= -2" target_x (rel (-2) (-3)) false (-2)

(* the exact case from XXX.0.3: the old `v-1' rule returned -1/2 for x<1/2 *)
let () =
  let f = rel (-2) 1 in
  let inner =
    match bridge_cert_rel_hi [] target_x ("rel", f, true) with
    | Some rest -> (match bridge_cert_hi (=) 0 [] [] rest with
        | Some (v,_) -> v
        | None -> failwith "no inner bound")
    | None -> failwith "no rest form" in
  let legacy = Num.sub_num inner num_1 in
  let tm = true_max1 target_x f true in
  Printf.printf "case x<1/2: inner bound=%s, legacy(v-1)=%s, true max=%s\n%!"
    (str inner) (str legacy) (str tm);
  check "the legacy (v-1) candidate for x<1/2 is unsound" (Num.lt_num legacy tm)

(* a strict step over a *rational-valued* form must not be rounded: the form
   y+1/2 under y<1/2 has true sup 1/2, so an over-rounded 0 would be wrong. *)
let () =
  let target = ([ (1, i 1) ], q 1 2) in
  let f = ([ (1, i (-2)) ], i 1) in
  match bridge_cert_hi (=) 3 [] [ (0, "rel", f, true) ] target with
  | Some (v,_) ->
      let sup = true_max1 target f true in
      let over = Num.sub_num (Num.ceiling_num v) num_1 in
      Printf.printf "rational form y+1/2, y<1/2: shipped=%s over-rounded=%s true_sup=%s\n%!"
        (str v) (str over) (str sup);
      check "a rational-valued form is not over-rounded" (Num.le_num sup v);
      check "over-rounding a rational-valued form is unsound" (Num.lt_num over sup)
  | None -> check "a rational-valued form is not over-rounded" false

(* --- 4. bridge_cert_lo via the negation -------------------------------- *)

let run_lo target f strict =
  bridge_cert_lo (=) 3 [] [ (0, "rel", f, strict) ] target

let check_lo_case name target f strict expected =
  match run_lo target f strict with
  | Some (v,_) ->
      let tm = true_min1 target f strict in
      if not (Num.eq_num v (i expected) && Num.eq_num v tm) then
        Printf.printf "  %s: got %s expected %d true_min %s\n%!"
          name (str v) expected (str tm);
      check name (Num.eq_num v (i expected) && Num.eq_num v tm)
  | None -> check name false

let () =
  (* f = 2x + c >=/> 0  <=>  x >=/> -c/2 *)
  check_lo_case "lo: 2x+1 > 0   -> x >= 0"  target_x (rel 2 1) true 0;
  check_lo_case "lo: 2x+1 >= 0  -> x >= 0"  target_x (rel 2 1) false 0;
  check_lo_case "lo: 2x+3 > 0   -> x >= -1" target_x (rel 2 3) true (-1);
  check_lo_case "lo: 2x+3 >= 0  -> x >= -1" target_x (rel 2 3) false (-1);
  check_lo_case "lo: 2x-1 > 0   -> x >= 1"  target_x (rel 2 (-1)) true 1;
  check_lo_case "lo: 2x-1 >= 0  -> x >= 1"  target_x (rel 2 (-1)) false 1

(* --- 5. the base table path (scaling and summation) -------------------- *)

let () =
  let tbl = [ (0, (None, Some (i 3,false,())));
              (1, (None, Some (i 5,false,()))) ] in
  let target = ([ (0, i 1); (1, i 1) ], i 0) in
  let ok = match bridge_cert_hi (=) 0 tbl [] target with
    | Some (v,_) -> Num.eq_num v (i 8)
    | None -> false in
  check "base: sum of upper bounds is 3+5=8" ok;
  let tbl2 = [ (0, (None, Some (i 3,false,())));
               (1, (Some (i (-2),false,()), None)) ] in
  let target2 = ([ (0, i 1); (1, i (-2)) ], i 0) in
  let ok2 = match bridge_cert_hi (=) 0 tbl2 [] target2 with
    | Some (v,_) -> Num.eq_num v (i 7)
    | None -> false in
  check "base: negative coefficient swaps the endpoint (-2)*(-2)+3=7" ok2

(* --- summary ------------------------------------------------------------ *)

let () =
  Printf.printf "\ncarry-cert bound checks: %d, failures: %d\n%!" !checks !failures;
  if !failures = 0 then begin
    Printf.printf "OK cert_bounds_selftest\n%!";
    exit 0
  end else begin
    Printf.printf "FAIL cert_bounds_selftest: %d failure(s)\n%!" !failures;
    exit 1
  end
