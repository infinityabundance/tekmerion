(* DIAGNOSTIC PROBE (not a proof, not part of any claim).
   Localises where each basic axiom enters the s2n-bignum x86 proof context.
   Built and run with the IX.57 axiom-stamp measurement kernel. *)

needs "x86/proofs/base.ml";;

let probe name th =
  let ids = String.concat "," (List.map string_of_int (stamp_of th)) in
  Printf.printf "PROBE %-28s ids=[%s]\n%!" name ids;;

(* 1. Library lemmas immediately visible in the base context. *)
probe "BIGNUM_FROM_MEMORY_BYTES" BIGNUM_FROM_MEMORY_BYTES;;
probe "C_ARGUMENTS" C_ARGUMENTS;;
probe "NONOVERLAPPING_CLAUSES" NONOVERLAPPING_CLAUSES;;
probe "SOME_FLAGS" SOME_FLAGS;;

(* 2. The ELF decode and the definition of this routine's modulus. *)
let bignum_neg_p25519_mc =
  define_assert_from_elf "bignum_neg_p25519_mc" "x86/curve25519/bignum_neg_p25519.o"
[
  0xf3; 0x0f; 0x1e; 0xfa;
  0x48; 0x8b; 0x06; 0x48;
  0x89; 0xc2; 0x48; 0x8b;
  0x4e; 0x08; 0x48; 0x09;
  0xca; 0x4c; 0x8b; 0x46;
  0x10; 0x4c; 0x09; 0xc2;
  0x4c; 0x8b; 0x4e; 0x18;
  0x4c; 0x09; 0xca; 0x48;
  0xf7; 0xda; 0x48; 0x19;
  0xd2; 0x49; 0xc7; 0xc2;
  0xed; 0xff; 0xff; 0xff;
  0x49; 0x21; 0xd2; 0x49;
  0x29; 0xc2; 0x4c; 0x89;
  0x17; 0x49; 0x89; 0xd2;
  0x49; 0x19; 0xca; 0x4c;
  0x89; 0x57; 0x08; 0x49;
  0x89; 0xd2; 0x4d; 0x19;
  0xc2; 0x4c; 0x89; 0x57;
  0x10; 0x4c; 0x19; 0xca;
  0x48; 0x0f; 0xba; 0xf2;
  0x3f; 0x48; 0x89; 0x57;
  0x18; 0xc3;
];;
probe "define_assert_from_elf" bignum_neg_p25519_mc;;

let bignum_neg_p25519_tmc = define_trimmed "bignum_neg_p25519_tmc" bignum_neg_p25519_mc;;
probe "define_trimmed" bignum_neg_p25519_tmc;;

let BIGNUM_NEG_P25519_EXEC = fst (X86_MK_CORE_EXEC_RULE bignum_neg_p25519_tmc);;
probe "X86_MK_CORE_EXEC_RULE" BIGNUM_NEG_P25519_EXEC;;

let p_25519 = new_definition `p_25519 = 57896044618658097711785492504343953926634992332820282019728792003956564819949`;;
probe "new_definition p_25519" p_25519;;

(* 3. Gradient points inside this context. *)
probe "REFL_int64" (REFL `x:int64`);;
probe "ARITH_RULE 2+2=4" (ARITH_RULE `2 + 2 = 4`);;
probe "MESON P==>P" (MESON [] `!P:bool. P ==> P`);;

Printf.printf "PROBE_DONE\n%!";;
