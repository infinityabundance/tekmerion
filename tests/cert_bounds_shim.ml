(* A tiny exact-rational implementation of the part of HOL Light's `Num'
   interface that the carry-certificate bound block of tools/bridge.ml uses
   (XXX.A2/A3/A4).

   The block between BEGIN/END BRIDGE_CERT_STANDALONE is extracted verbatim and
   compiled against this shim, so the shipped numeric arithmetic is tested
   outside HOL Light.  Rationals (not ints) are used so that a non-exact
   division has its real meaning: `Num.div_num 1 2` is `1/2`, and the endpoint
   rules must round it, never silently truncate it.

   The three bridge_lf helpers below are exact copies of the shipped
   definitions (tools/bridge.ml, 6c.1); they are here only because the cert
   block calls them.  If those helpers change, this shim must be updated in the
   same commit.  The three functions under test are the extracted block itself,
   not a copy. *)

module Num = struct
  type num = { n : int; d : int }          (* d > 0, reduced *)

  let rec gcd a b = if b = 0 then abs a else gcd b (a mod b)

  let make n d =
    if d = 0 then invalid_arg "Num.make: zero denominator";
    let s = if d < 0 then -1 else 1 in
    let g = gcd n d in
    let g = if g = 0 then 1 else g in
    { n = s * n / g; d = s * d / g }

  let int_num k = make k 1
  let num_0 = int_num 0
  let num_1 = int_num 1

  let add_num a b = make (a.n * b.d + b.n * a.d) (a.d * b.d)
  let minus_num a = { n = -a.n; d = a.d }
  let sub_num a b = add_num a (minus_num b)
  let mult_num a b = make (a.n * b.n) (a.d * b.d)
  let div_num a b = make (a.n * b.d) (a.d * b.n)
  let abs_num a = { n = abs a.n; d = a.d }

  let lt_num a b = a.n * b.d < b.n * a.d
  let le_num a b = not (lt_num b a)
  let eq_num a b = a.n * b.d = b.n * a.d

  let floor_num a =
    let q = a.n / a.d in
    if a.n < 0 && a.n mod a.d <> 0 then int_num (q - 1) else int_num q

  let ceiling_num a =
    let q = a.n / a.d in
    if a.n > 0 && a.n mod a.d <> 0 then int_num (q + 1) else int_num q

  let is_integer_num a = a.d = 1

  let to_string a =
    if a.d = 1 then string_of_int a.n else Printf.sprintf "%d/%d" a.n a.d
end

let num_0 = Num.num_0
let num_1 = Num.num_1

(* atoms are small integers in the standalone test; this is the only piece of
   the HOL term world the cert block touches. *)
let aconv (a:int) b = a = b

(* exact copies of the shipped linear-form helpers (tools/bridge.ml 6c.1) *)
let bridge_lf_coeff t xs =
  try snd (List.find (fun (u,_) -> aconv u t) xs) with Not_found -> num_0

let rec bridge_lf_addc xs t k =
  match xs with
  | [] -> if Num.eq_num k num_0 then [] else [(t,k)]
  | (u,k0)::rest ->
      if aconv u t then
        (let k' = Num.add_num k0 k in
         if Num.eq_num k' num_0 then rest else (u,k')::rest)
      else (u,k0) :: bridge_lf_addc rest t k

let bridge_lf_add (xs,c) (ys,d) =
  let xs' = List.fold_left (fun a (t,k) -> bridge_lf_addc a t k) xs ys in
  (xs', Num.add_num c d)

let bridge_lf_scale k (xs,c) =
  if Num.eq_num k num_0 then ([], num_0)
  else (List.map (fun (t,j) -> (t, Num.mult_num k j)) xs, Num.mult_num k c)

let bridge_lf_neg f = bridge_lf_scale (Num.minus_num num_1) f
