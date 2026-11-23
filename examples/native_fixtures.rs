//! Offline fixture generator for the native checker.
//!
//! This binary *proposes* CNF and certificate bytes. It is not part of the checker
//! path and is never invoked by `cargo test`.
//!
//! ```text
//! cargo run --offline --example native_fixtures -- cnf add_comm_8 out.cnf
//! cadical --lrat --binary=false --plain out.cnf out.lrat
//! cargo run --offline --example native_fixtures -- cert add_comm_8 out.cnf out.lrat out.cert
//! ```

use std::env;
use std::fs;
use std::time::Instant;

use tekmerion::native::{bitblast, check, Blob, BlobKind, Certificate, Step, Term};

fn var(width: u32, name: &str) -> Term {
    Term::var(width, name).expect("valid variable")
}

fn proposition(name: &str) -> Term {
    match name {
        // 8-bit addition is commutative.
        "add_comm_8" => {
            let x = var(8, "x");
            let y = var(8, "y");
            let lhs = Term::add(x.clone(), y.clone()).expect("well-sorted");
            let rhs = Term::add(y, x).expect("well-sorted");
            Term::eq(lhs, rhs).expect("well-sorted")
        }
        // 64-bit: (x + y) - y = x. The side condition is vacuous under modulo
        // 2^64 semantics; the carry out is discarded by definition.
        "sub_add_64" => {
            let x = var(64, "x");
            let y = var(64, "y");
            let sum = Term::add(x.clone(), y.clone()).expect("well-sorted");
            let dif = Term::sub(sum, y).expect("well-sorted");
            Term::eq(dif, x).expect("well-sorted")
        }
        other => panic!("unknown identity {other}"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: native_fixtures cnf <name> <out.cnf> | cert <name> <cnf> <lrat> <out.cert> | check <cert> [reps]");
        std::process::exit(2);
    }
    match args[1].as_str() {
        "cnf" => {
            let prop = proposition(&args[2]);
            let cnf = bitblast(&prop).expect("bit-blast");
            fs::write(&args[3], cnf.to_dimacs()).expect("write CNF");
            println!("{}", prop.render());
        }
        "cert" => {
            let prop = proposition(&args[2]);
            let cnf_bytes = fs::read(&args[3]).expect("read CNF");
            let lrat_bytes = fs::read(&args[4]).expect("read LRAT");
            let cnf = Blob::new(BlobKind::Cnf, cnf_bytes);
            let lrat = Blob::new(BlobKind::Lrat, lrat_bytes);
            let step = Step::bitblast(0, cnf.digest(), lrat.digest(), prop.clone());
            let cert = Certificate::new(0, vec![step], vec![cnf, lrat]).expect("valid certificate");
            fs::write(&args[5], cert.render()).expect("write certificate");
            println!("{}", prop.render());
        }
        "check" => {
            let text = fs::read_to_string(&args[2]).expect("read certificate");
            let repetitions: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
            let cert = Certificate::parse(&text).expect("certificate parses");
            let start = Instant::now();
            let mut theorem = None;
            for _ in 0..repetitions {
                theorem = Some(check(&cert).expect("certificate checks"));
            }
            let elapsed = start.elapsed();
            let theorem = theorem.expect("at least one repetition");
            println!("proposition: {}", theorem.render());
            println!("digest: {}", theorem.digest());
            println!(
                "checks: {} total: {:?} mean: {:?}",
                repetitions,
                elapsed,
                elapsed / repetitions
            );
        }
        other => panic!("unknown mode {other}"),
    }
}
