(* A tiny exact-rational implementation of the part of HOL Light's `Num'
   interface that the interval block of tools/bridge.ml uses (XIX.A3).

   The extracted block is compiled against this shim, so the shipped bound
   arithmetic is tested verbatim outside HOL Light.  Rationals (not ints) are
   used so that strict endpoints and half-unit samples have their real meaning:
   x > v is a strict bound, not "the next integer". *)

module Num = struct
  type num = { n : int; d : int }

  let rec gcd a b = if b = 0 then abs a else gcd b (a mod b)

  let make n d =
    if d = 0 then invalid_arg "Num.make: zero denominator";
    let s = if d < 0 then -1 else 1 in
    let g = gcd n d in
    { n = s * n / g; d = s * d / g }

  let num_0 = make 0 1
  let int_num k = make k 1

  let add_num a b = make (a.n * b.d + b.n * a.d) (a.d * b.d)
  let mult_num a b = make (a.n * b.n) (a.d * b.d)
  let lt_num a b = a.n * b.d < b.n * a.d
  let le_num a b = not (lt_num b a)
  let eq_num a b = a.n * b.d = b.n * a.d
  let neg_num a = { a with n = -a.n }

  let to_string a = if a.d = 1 then string_of_int a.n
                    else Printf.sprintf "%d/%d" a.n a.d
end

let num_0 = Num.num_0
