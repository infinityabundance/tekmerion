//! The checker and the only constructor of [`Theorem`].
//!
//! There is no process spawning, no filesystem access, and no network access on
//! this path. Input is a [`Certificate`]; output is either an error or a
//! [`Theorem`] whose proposition and digest are fixed by the certificate.
//!
//! The five rules are the whole trusted calculus:
//!
//! * `refl`  — `t = t`;
//! * `sym`   — symmetry of equality;
//! * `trans` — transitivity of equality;
//! * `congr` — replacing one operand of `add`/`sub` by an equal term;
//! * `bitblast` — a Boolean proposition is closed when its canonical bit-blast is
//!   refuted by a checked RUP LRAT derivation.
//!
//! A theorem is not a claim about the certificate; it is the exact proposition
//! derived by the named root step, together with a digest of that proposition.

use std::collections::BTreeMap;
use std::fmt;

use crate::native::bitblast::{bitblast, Cnf};
use crate::native::cert::{BlobKind, Certificate, Rule};
use crate::native::lrat;
use crate::native::{Error, LratError, Term};
use crate::{Canon, Digest};

/// A proposition that this checker has derived.
///
/// Construction is private to this module: the only way to obtain a `Theorem` is
/// [`check`].
#[derive(Clone, PartialEq, Eq)]
pub struct Theorem {
    proposition: Term,
    digest: Digest,
}

impl Theorem {
    /// The exact proposition derived, in canonical form.
    pub fn proposition(&self) -> &Term {
        &self.proposition
    }

    /// Content digest of the proposition.
    pub fn digest(&self) -> Digest {
        self.digest
    }

    pub fn render(&self) -> String {
        self.proposition.render()
    }

    fn new(proposition: Term) -> Self {
        let mut c = Canon::new("tekmerion/native/theorem/v1");
        c.digest(&proposition.digest());
        Theorem {
            proposition,
            digest: c.finish(),
        }
    }
}

impl fmt::Debug for Theorem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Theorem")
            .field("proposition", &self.render())
            .field("digest", &self.digest.hex())
            .finish()
    }
}

impl fmt::Display for Theorem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.render(), self.digest)
    }
}

/// Check every step of `cert` and return the theorem named by its root step.
pub fn check(cert: &Certificate) -> Result<Theorem, Error> {
    let order = cert.topological_order()?;
    let mut proved: BTreeMap<u32, Term> = BTreeMap::new();
    for id in order {
        let step = cert.step(id).ok_or(Error::DanglingRef(id))?;
        let prop = apply_rule(cert, step, &proved)?;
        proved.insert(id, prop);
    }
    let prop = proved
        .get(&cert.theorem_id())
        .ok_or(Error::MissingTheorem(cert.theorem_id()))?
        .clone();
    Ok(Theorem::new(prop))
}

fn apply_rule(
    cert: &Certificate,
    step: &crate::native::Step,
    proved: &BTreeMap<u32, Term>,
) -> Result<Term, Error> {
    match step.rule() {
        Rule::Refl => Ok(step.conclusion().clone()),
        Rule::Sym => {
            let premise = premise(proved, step.premises()[0])?;
            let (a, b) =
                as_eq(premise).ok_or(Error::RuleMismatch("sym premise is not an equality"))?;
            let (x, y) = as_eq(step.conclusion())
                .ok_or(Error::RuleMismatch("sym conclusion is not an equality"))?;
            if x == b && y == a {
                Ok(step.conclusion().clone())
            } else {
                Err(Error::RuleMismatch(
                    "sym conclusion is not the premise with sides exchanged",
                ))
            }
        }
        Rule::Trans => {
            let first = premise(proved, step.premises()[0])?;
            let second = premise(proved, step.premises()[1])?;
            let (a, b) = as_eq(first).ok_or(Error::RuleMismatch(
                "trans first premise is not an equality",
            ))?;
            let (c, d) = as_eq(second).ok_or(Error::RuleMismatch(
                "trans second premise is not an equality",
            ))?;
            if b != c {
                return Err(Error::RuleMismatch(
                    "trans premises do not share a middle term",
                ));
            }
            let (x, y) = as_eq(step.conclusion())
                .ok_or(Error::RuleMismatch("trans conclusion is not an equality"))?;
            if x == a && y == d {
                Ok(step.conclusion().clone())
            } else {
                Err(Error::RuleMismatch(
                    "trans conclusion is not the outer pair",
                ))
            }
        }
        Rule::Congr => {
            let premise = premise(proved, step.premises()[0])?;
            let (a, b) =
                as_eq(premise).ok_or(Error::RuleMismatch("congr premise is not an equality"))?;
            let (left, right) = as_eq(step.conclusion())
                .ok_or(Error::RuleMismatch("congr conclusion is not an equality"))?;
            if congruent(a, b, left, right) {
                Ok(step.conclusion().clone())
            } else {
                Err(Error::RuleMismatch(
                    "congr does not replace exactly one operand of add/sub",
                ))
            }
        }
        Rule::Bitblast { cnf, lrat } => {
            let cnf_blob = cert.blob(*cnf, BlobKind::Cnf)?;
            let lrat_blob = cert.blob(*lrat, BlobKind::Lrat)?;
            let attached = Cnf::parse_dimacs(cnf_blob.bytes())?;
            let expected = bitblast(step.conclusion())?;
            if attached != expected {
                return Err(Error::CnfMismatch);
            }
            let text = std::str::from_utf8(lrat_blob.bytes()).map_err(|_| {
                Error::Lrat(LratError::Malformed {
                    line: 1,
                    detail: "LRAT payload is not UTF-8",
                })
            })?;
            lrat::check(&expected, text)?;
            Ok(step.conclusion().clone())
        }
    }
}

fn premise(proved: &BTreeMap<u32, Term>, id: u32) -> Result<&Term, Error> {
    proved.get(&id).ok_or(Error::DanglingRef(id))
}

fn as_eq(term: &Term) -> Option<(&Term, &Term)> {
    match term {
        Term::Eq { left, right, .. } => Some((left, right)),
        _ => None,
    }
}

/// One-operand congruence for `add`/`sub`.
fn congruent(a: &Term, b: &Term, left: &Term, right: &Term) -> bool {
    match (left, right) {
        (
            Term::Add {
                width: lw,
                left: ll,
                right: lr,
            },
            Term::Add {
                width: rw,
                left: rl,
                right: rr,
            },
        )
        | (
            Term::Sub {
                width: lw,
                left: ll,
                right: lr,
            },
            Term::Sub {
                width: rw,
                left: rl,
                right: rr,
            },
        ) => {
            lw == rw
                && ((ll.as_ref() == a && rl.as_ref() == b && lr == rr)
                    || (lr.as_ref() == a && rr.as_ref() == b && ll == rl))
        }
        _ => false,
    }
}
