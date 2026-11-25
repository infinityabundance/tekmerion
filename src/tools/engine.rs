//! Deterministic selection policy for external tools (XIX.B1, XIX.B2).
//!
//! This module is **pure**.  It takes a supplied candidate list — a tool name
//! together with *every* executable path found for that name — plus a probe
//! closure, and returns the first candidate the probe accepts.  It performs no
//! filesystem access, spawns no process and consults no clock, so the policy
//! tests cannot race a freshly written executable, and a defective shim cannot
//! shadow a usable binary later in the search path: every path is probed, not
//! just the first one per name.
//!
//! `disasm` owns the two impure halves: it enumerates the paths
//! (`which_all_in`) and supplies the real probe (`probe_candidate`).

use std::path::{Path, PathBuf};

/// One tool name and every executable path found for it, in search order.
/// An empty `paths` means the name was not found; the policy still reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateName {
    pub name: String,
    pub paths: Vec<PathBuf>,
}

/// The first candidate the probe accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// There was no candidate name to consider at all (e.g. the caller passed
    /// only empty names).
    NoCandidate,
    /// Candidates were considered and every one was unusable.  `notes` names
    /// each path with what the probe reported, in search order.
    NoWorking { notes: Vec<String> },
}

/// Walk the candidate list in order and return the first path the probe
/// accepts.  `probe` returns `Err(reason)` for an unusable candidate; the
/// reason is recorded in the failure notes verbatim.
pub fn select<F>(names: &[CandidateName], mut probe: F) -> Result<Selection, SelectionError>
where
    F: FnMut(&Path) -> Result<(), String>,
{
    let mut notes: Vec<String> = Vec::new();
    let mut probed_any = false;
    for candidate in names {
        if candidate.paths.is_empty() {
            // Every name gets its own note, including one that follows a name
            // with usable paths, so the error is complete.
            notes.push(format!("{}: not found on PATH", candidate.name));
            continue;
        }
        for path in &candidate.paths {
            probed_any = true;
            match probe(path) {
                Ok(()) => {
                    return Ok(Selection {
                        name: candidate.name.clone(),
                        path: path.clone(),
                    })
                }
                Err(why) => notes.push(format!("{} ({why})", path.display())),
            }
        }
    }
    if !probed_any && notes.is_empty() {
        return Err(SelectionError::NoCandidate);
    }
    Err(SelectionError::NoWorking { notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(name: &str, paths: &[&str]) -> CandidateName {
        CandidateName {
            name: name.to_string(),
            paths: paths.iter().map(PathBuf::from).collect(),
        }
    }

    /// The defect B1 names: a bad first path for a name must not hide a good
    /// path for the *same* name later in the search path.
    #[test]
    fn every_path_for_a_name_is_probed() {
        let names = vec![c("objdump", &["/a/bad", "/b/good"])];
        let mut seen: Vec<String> = Vec::new();
        let got = select(&names, |p| {
            seen.push(p.display().to_string());
            if p.ends_with("good") {
                Ok(())
            } else {
                Err("decoded no `add`".to_string())
            }
        })
        .expect("the second path is usable");
        assert_eq!(got.path, PathBuf::from("/b/good"));
        assert_eq!(seen, vec!["/a/bad".to_string(), "/b/good".to_string()]);
    }

    #[test]
    fn later_names_are_probed_after_an_exhausted_name() {
        let names = vec![c("first", &["/x/one"]), c("second", &["/y/two"])];
        let got = select(&names, |p| {
            if p == Path::new("/y/two") {
                Ok(())
            } else {
                Err("unusable".to_string())
            }
        })
        .expect("the second name is usable");
        assert_eq!(got.name, "second");
        assert_eq!(got.path, PathBuf::from("/y/two"));
    }

    #[test]
    fn no_candidate_at_all_is_reported_as_such() {
        assert_eq!(select(&[], |_| Ok(())), Err(SelectionError::NoCandidate));
    }

    #[test]
    fn a_missing_name_is_named_in_the_notes() {
        let names = vec![c("ghost", &[]), c("also-ghost", &[])];
        match select(&names, |_| Ok(())) {
            Err(SelectionError::NoWorking { notes }) => {
                assert_eq!(notes[0], "ghost: not found on PATH");
                assert_eq!(notes[1], "also-ghost: not found on PATH");
            }
            other => panic!("expected NoWorking, got {other:?}"),
        }
    }

    #[test]
    fn every_rejection_reason_is_kept_in_order() {
        let names = vec![c("a", &["/p/one", "/p/two"])];
        match select(&names, |_| Err("nope".to_string())) {
            Err(SelectionError::NoWorking { notes }) => {
                assert_eq!(notes, vec!["/p/one (nope)", "/p/two (nope)"]);
            }
            other => panic!("expected NoWorking, got {other:?}"),
        }
    }

    /// Purity: the policy never stats the paths it is given, so a list of
    /// paths that do not exist is still decided by the probe alone.  This is
    /// what makes the policy tests independent of filesystem timing.
    #[test]
    fn policy_does_not_touch_the_filesystem() {
        let names = vec![c("virtual", &["/definitely/not/here", "/nor/here"])];
        let got = select(&names, |p| {
            if p.ends_with("here") && p.to_string_lossy().contains("nor") {
                Ok(())
            } else {
                Err("no".to_string())
            }
        })
        .expect("the probe decides");
        assert_eq!(got.path, PathBuf::from("/nor/here"));
    }
}
