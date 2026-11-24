//! Derive the accumulating step set from carry dataflow.
//!
//! This module is deliberately blind to any proof script. [`derive_accumulate`]
//! accepts only the parsed machine-code step list and a mnemonic -> semantics
//! table. It has no file I/O, no network, no subprocess, and it cannot see a
//! tactic line, a state-name list, a hand-written accumulate list, or the name
//! of a curve or a theorem. The only inputs are the machine instructions and
//! what those instructions mean.
//!
//! An instruction participates in a carry chain when it computes `d = a +/- b`
//! (optionally plus/minus an incoming carry flag). Such a chain can only be
//! *used* if every carry-in it consumes was produced by an earlier flag write
//! and every carry-out it produces is consumed by a later flag read, so those
//! are the liveness conditions checked here. Two further rules matter:
//!
//!  * the instruction must read at least one real register: one whose sources
//!    are all zero registers or immediates is a carry *materialisation*
//!    (`ADC Xd, XZR, XZR`), not a digit sum;
//!  * the instruction must either write a general-purpose register or be a
//!    *flag-only* add/sub form (`CMP`/`CMN`, or `ADCS`/`SBCS` to `XZR`), and a
//!    flag-only form is a link only if its carry-out is read as a carry-*in* by
//!    a later add/sub that does write a register;
//!  * a flag-only add/sub is itself excluded when it has no real register
//!    source, because then it constrains no word at all.
//!
//! `SP` as a destination is neither a value write for accumulation nor a
//! flag-only form, so it is excluded.
//!
//! The result is a sorted list of 1-based step numbers, matching the numbering
//! the ARM simulation uses (instruction 1 is the first word in the ELF `.text`).

use std::collections::BTreeMap;

/// One machine-code instruction, as parsed from an ELF disassembly.
///
/// `index` is 1-based and follows program order. `operands` are lower-cased
/// and stripped but otherwise as the disassembler emitted them (memory
/// operands keep their brackets), so `x4`, `xzr`, `#0x1` and `[x1, #16]` are
/// all distinguishable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub index: usize,
    pub mnemonic: String,
    pub operands: Vec<String>,
    pub raw: String,
    pub address: i64,
}

impl Step {
    pub fn new(index: usize, mnemonic: &str, operands: Vec<String>) -> Step {
        Step {
            index,
            mnemonic: mnemonic.to_string(),
            operands,
            raw: String::new(),
            address: 0,
        }
    }
}

/// The arithmetic form an add/sub instruction takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Add,
    Sub,
}

/// What one mnemonic does to registers and the carry flag.
///
/// `form`          `Add`, `Sub` or `None` (not an add/sub form).
/// `dest`          operand index written, or `None` (flag-only compare).
/// `sources`       operand indices that are read as values.
/// `reads_carry`   consumes the incoming carry flag.
/// `writes_carry`  produces a carry flag.
/// `reads_flags`   consumes flags for a conditional operation or branch.
/// `self_carry_materialisation`
///                 the two-operand form `d = d +/- d +/- carry` copies the
///                 carry into `d` instead of summing two digits (x86 `ADC r,r`
///                 / `SBB r,r`).
/// `always_link`   a value-writing add/sub is a digit sum whether or not its
///                 carry-out is later read (the dead-carry top limb).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSemantics {
    pub form: Option<Form>,
    pub dest: Option<usize>,
    pub sources: &'static [usize],
    pub reads_carry: bool,
    pub writes_carry: bool,
    pub reads_flags: bool,
    pub self_carry_materialisation: bool,
    pub always_link: bool,
}

/// A semantics entry that is not an add/sub form at all.
pub const NO_SEMANTICS: InstructionSemantics = InstructionSemantics {
    form: None,
    dest: None,
    sources: &[],
    reads_carry: false,
    writes_carry: false,
    reads_flags: false,
    self_carry_materialisation: false,
    always_link: false,
};

const fn sem(form: Form) -> InstructionSemantics {
    InstructionSemantics {
        form: Some(form),
        ..NO_SEMANTICS
    }
}

const fn with_dest(mut s: InstructionSemantics, dest: usize) -> InstructionSemantics {
    s.dest = Some(dest);
    s
}

const fn with_sources(
    mut s: InstructionSemantics,
    sources: &'static [usize],
) -> InstructionSemantics {
    s.sources = sources;
    s
}

const fn reads_carry(mut s: InstructionSemantics) -> InstructionSemantics {
    s.reads_carry = true;
    s
}

const fn writes_carry(mut s: InstructionSemantics) -> InstructionSemantics {
    s.writes_carry = true;
    s
}

const fn reads_flags(mut s: InstructionSemantics) -> InstructionSemantics {
    s.reads_flags = true;
    s
}

const fn self_materialising(mut s: InstructionSemantics) -> InstructionSemantics {
    s.self_carry_materialisation = true;
    s
}

const fn always_link(mut s: InstructionSemantics) -> InstructionSemantics {
    s.always_link = true;
    s
}

/// AArch64 add/sub-with-carry semantics.
///
/// Only the ARM64 add/sub-with-carry family is modelled here; every other
/// mnemonic is a non-candidate for accumulation. The table is data, not
/// policy: the policy (liveness, zero-register exclusions) lives in
/// [`derive_accumulate`].
pub fn default_semantics() -> BTreeMap<&'static str, InstructionSemantics> {
    let add2 = with_sources(with_dest(sem(Form::Add), 0), &[1, 2]);
    let sub2 = with_sources(with_dest(sem(Form::Sub), 0), &[1, 2]);
    let sub1 = with_sources(with_dest(sem(Form::Sub), 0), &[1]);
    let cmp2 = with_sources(sem(Form::Sub), &[0, 1]);
    let cmn2 = with_sources(sem(Form::Add), &[0, 1]);
    let mut m = BTreeMap::new();
    // d = a + b            (no carry in, writes C)
    m.insert("adds", writes_carry(add2));
    // d = a + b + C        (carry in; +S writes C out)
    m.insert("adcs", writes_carry(reads_carry(add2)));
    m.insert("adc", reads_carry(add2));
    // d = a - b            (no carry in, writes C)
    m.insert("subs", writes_carry(sub2));
    // d = a - b - C        (carry in; +S writes C out)
    m.insert("sbcs", writes_carry(reads_carry(sub2)));
    m.insert("sbc", reads_carry(sub2));
    // d = 0 - b - C        (the "negate with carry" forms)
    m.insert("ngcs", writes_carry(reads_carry(sub1)));
    m.insert("ngc", reads_carry(sub1));
    // flag-only probes: value written to XZR, so they never contribute a digit
    m.insert("cmp", writes_carry(cmp2));
    m.insert("cmn", writes_carry(cmn2));
    m.insert("ccmp", writes_carry(reads_flags(reads_carry(cmp2))));
    m.insert("ccmn", writes_carry(reads_flags(reads_carry(cmn2))));
    // Non-arithmetic flag writers: they matter only as chain boundaries.
    for name in ["ands", "orrs", "eors", "bics", "negs", "tst"] {
        m.insert(name, writes_carry(NO_SEMANTICS));
    }
    m
}

/// The documented meaning of the x86 CF flag after each arithmetic form.
///
/// Two differences from AArch64 are load-bearing.
///
///  1. The flag semantics are inverted for subtraction. On AArch64 the C flag
///     after SUBS/SBCS is "no borrow"; on x86 the CF flag after SUB/SBB/NEG/CMP
///     is "borrow". The accumulator relation has the same real shape on both,
///     but the meaning of its `carry_sN` bit is the opposite.
///  2. Every x86 ADD/SUB/ADC/SBB writes the flags unconditionally (there is no
///     `S` suffix). The two-operand form is `d = d +/- src`.
pub fn x86_cf_meaning() -> BTreeMap<&'static str, &'static str> {
    let mut m = BTreeMap::new();
    m.insert(
        "add",
        "carry (unsigned overflow): set iff val d + val s + c >= 2^64",
    );
    m.insert(
        "adc",
        "carry (unsigned overflow): set iff val d + val s + c >= 2^64",
    );
    m.insert("sub", "borrow: set iff val d < val s");
    m.insert("sbb", "borrow: set iff val d < val s + c");
    m.insert("neg", "borrow: set iff val d != 0");
    m.insert("cmp", "borrow: set iff val d < val s");
    m
}

/// x86-64 add/sub semantics, in Intel operand order (destination first).
pub fn x86_semantics() -> BTreeMap<&'static str, InstructionSemantics> {
    let acc2 = with_sources(with_dest(sem(Form::Add), 0), &[0, 1]);
    let sbb2 = with_sources(with_dest(sem(Form::Sub), 0), &[0, 1]);
    let neg1 = with_sources(with_dest(sem(Form::Sub), 0), &[0]);
    let cmp2 = with_sources(sem(Form::Sub), &[0, 1]);
    let mut m = BTreeMap::new();
    // d = d + s
    m.insert("add", always_link(writes_carry(acc2)));
    // d = d + s + CF; ADC r,r copies CF into r
    m.insert(
        "adc",
        always_link(writes_carry(reads_carry(self_materialising(acc2)))),
    );
    // d = d - s
    m.insert("sub", always_link(writes_carry(sbb2)));
    // d = d - s - CF; SBB r,r copies -CF into r
    m.insert(
        "sbb",
        always_link(writes_carry(reads_carry(self_materialising(sbb2)))),
    );
    // d = 0 - d  (CF is the borrow, i.e. d != 0)
    m.insert("neg", always_link(writes_carry(neg1)));
    // flag-only forms: no value destination
    m.insert("cmp", writes_carry(cmp2));
    // CF-only writers that are not value adds/subs
    for name in ["stc", "clc", "cmc"] {
        m.insert(name, writes_carry(NO_SEMANTICS));
    }
    // Logical forms clear CF (and OF), so they kill an earlier carry-out.
    for name in ["and", "or", "xor", "test"] {
        m.insert(name, writes_carry(NO_SEMANTICS));
    }
    // ADCX reads and writes CF only; ADOX drives OF, not CF, and is absent.
    m.insert(
        "adcx",
        always_link(writes_carry(reads_carry(self_materialising(acc2)))),
    );
    m
}

/// Mnemonics that consume flags without being add/sub forms.
const FLAG_READERS: &[&str] = &[
    "csel", "csinc", "csinv", "csneg", "cset", "csetm", "cinc", "cinv", "cneg",
];

/// Conditional-branch suffixes.
pub const COND_SUFFIXES: &[&str] = &[
    "eq", "ne", "cs", "hs", "cc", "lo", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le",
    "al",
];

fn is_conditional_branch(mnemonic: &str) -> bool {
    match mnemonic.strip_prefix("b.") {
        Some(suffix) => COND_SUFFIXES.contains(&suffix),
        None => false,
    }
}

fn is_flag_writer(sem: &InstructionSemantics) -> bool {
    sem.writes_carry
}

fn is_flag_reader(mnemonic: &str, sem: &InstructionSemantics) -> bool {
    sem.reads_flags
        || sem.reads_carry
        || FLAG_READERS.contains(&mnemonic)
        || is_conditional_branch(mnemonic)
}

/// x86-64 general-purpose register names in all widths.
const X86_GPRS: &[&str] = &[
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp", "eax", "ebx", "ecx", "edx", "esi",
    "edi", "ebp", "esp", "ax", "bx", "cx", "dx", "si", "di", "bp", "sp", "al", "ah", "bl", "bh",
    "cl", "ch", "dl", "dh", "sil", "dil", "bpl", "spl", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15", "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d", "r15d", "r8w", "r9w",
    "r10w", "r11w", "r12w", "r13w", "r14w", "r15w", "r8b", "r9b", "r10b", "r11b", "r12b", "r13b",
    "r14b", "r15b",
];

/// True for a general-purpose register of either architecture.
///
/// AArch64: `x0..x30` / `w0..w30`; `xzr`/`wzr`/`sp`/`wsp` are not value
/// registers. x86-64: the 64/32/16/8-bit names of RAX..R15, with RSP excluded
/// for the same reason as SP. Memory operands (they contain `[`) and
/// immediates are excluded.
pub fn is_general_register(operand: &str) -> bool {
    if operand.is_empty() {
        return false;
    }
    if operand.contains('[') {
        return false;
    }
    if matches!(operand, "xzr" | "wzr" | "sp" | "wsp") {
        return false;
    }
    // AArch64
    if let Some(first) = operand.chars().next() {
        if first == 'x' || first == 'w' {
            let digits = &operand[1..];
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(n) = digits.parse::<u32>() {
                    if n <= 30 {
                        return true;
                    }
                }
            }
        }
    }
    // x86-64
    if !matches!(operand, "rsp" | "esp" | "sp" | "spl") && X86_GPRS.contains(&operand) {
        return true;
    }
    false
}

const ZERO_REGISTERS: &[&str] = &["xzr", "wzr"];

fn dest_operand<'a>(step: &'a Step, sem: &InstructionSemantics) -> Option<&'a str> {
    let i = sem.dest?;
    step.operands.get(i).map(String::as_str)
}

fn writes_value(step: &Step, sem: &InstructionSemantics) -> bool {
    match dest_operand(step, sem) {
        Some(dest) => is_general_register(dest),
        None => false,
    }
}

fn is_flag_only(step: &Step, sem: &InstructionSemantics) -> bool {
    match dest_operand(step, sem) {
        None => true,
        Some(dest) => ZERO_REGISTERS.contains(&dest),
    }
}

fn source_operands<'a>(step: &'a Step, sem: &InstructionSemantics) -> Vec<&'a str> {
    sem.sources
        .iter()
        .filter_map(|&k| step.operands.get(k).map(String::as_str))
        .collect()
}

fn is_self_carry_materialisation(step: &Step, sem: &InstructionSemantics) -> bool {
    if !sem.self_carry_materialisation {
        return false;
    }
    let dest = match dest_operand(step, sem) {
        Some(d) => d,
        None => return false,
    };
    let sources = source_operands(step, sem);
    !sources.is_empty() && sources.iter().all(|o| *o == dest)
}

/// Is the incoming flag of step `i` produced by an earlier flag write?
///
/// A flag writer starts a new definition and therefore grounds the carry-in; a
/// flag reader is transparent to the definition, so the search continues past
/// `ADDS; ADC; ADCS`. Only running off the start of the program leaves the
/// carry-in ungrounded.
fn carry_in_is_live(sem: &[InstructionSemantics], i: usize) -> bool {
    (0..i).rev().any(|j| is_flag_writer(&sem[j]))
}

/// Is the flag produced by step `i` consumed later before being overwritten?
///
/// A reader is checked before a writer: the `S` forms both consume the incoming
/// flag as carry-in and produce a new one, so an `ADCS` directly after an
/// `ADDS` does consume the `ADDS` carry-out.
fn carry_out_is_live(steps: &[Step], sem: &[InstructionSemantics], i: usize) -> bool {
    for j in (i + 1)..steps.len() {
        if is_flag_reader(&steps[j].mnemonic, &sem[j]) {
            return true;
        }
        if is_flag_writer(&sem[j]) {
            return false;
        }
    }
    false
}

/// Does step `i`'s carry-out reach a later add/sub that writes a register?
///
/// This is the liveness condition for a *flag-only* form. A bare flag reader
/// (`CSEL`/`CSETM`) is not counted: it consumes the flag as a predicate, so
/// relating the flag to a word buys nothing. The chain continues through
/// flag-only add/sub forms and is killed by any other flag writer.
fn carry_out_feeds_chain(steps: &[Step], sem: &[InstructionSemantics], i: usize) -> bool {
    for j in (i + 1)..steps.len() {
        let s = &sem[j];
        if matches!(s.form, Some(Form::Add | Form::Sub)) && s.reads_carry {
            if writes_value(&steps[j], s) {
                return true;
            }
            if s.writes_carry {
                continue;
            }
            return false;
        }
        if is_flag_writer(s) {
            return false;
        }
    }
    false
}

/// Return the sorted 1-based accumulating step numbers for `exec_steps` under
/// the AArch64 semantics table.
pub fn derive_accumulate(exec_steps: &[Step]) -> Vec<usize> {
    derive_accumulate_with(exec_steps, &default_semantics())
}

/// Return the sorted 1-based accumulating step numbers for `exec_steps`.
///
/// The decision procedure is described in the module docstring. It is total:
/// every step list yields a list (possibly empty), and an unknown mnemonic is
/// simply not an add/sub form.
pub fn derive_accumulate_with(
    exec_steps: &[Step],
    semantics: &BTreeMap<&'static str, InstructionSemantics>,
) -> Vec<usize> {
    let steps = exec_steps;
    let sem: Vec<InstructionSemantics> = steps
        .iter()
        .map(|s| {
            semantics
                .get(s.mnemonic.as_str())
                .copied()
                .unwrap_or(NO_SEMANTICS)
        })
        .collect();

    let mut derived = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        let s = &sem[i];
        if !matches!(s.form, Some(Form::Add | Form::Sub)) {
            continue;
        }

        // 1. Either it writes a general-purpose register, or it is a flag-only
        //    form (whose own claim to a relation is checked in 3).
        let val = writes_value(step, s);
        let flag_only = is_flag_only(step, s);
        if !(val || flag_only) {
            continue;
        }

        // 2. It must have at least one real register source.
        if is_self_carry_materialisation(step, s) {
            continue;
        }
        if !source_operands(step, s)
            .iter()
            .any(|o| is_general_register(o))
        {
            continue;
        }

        // 3. Carry/flag dataflow must justify taking an accumulator relation.
        if s.reads_carry && !carry_in_is_live(&sem, i) {
            continue;
        }
        if flag_only {
            if !carry_out_feeds_chain(steps, &sem, i) {
                continue;
            }
        } else if s.writes_carry && !s.always_link && !carry_out_is_live(steps, &sem, i) {
            continue;
        }

        derived.push(step.index);
    }

    derived.sort_unstable();
    derived
}
