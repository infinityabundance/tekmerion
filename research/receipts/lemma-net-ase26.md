# Receipt — LemmaNet (ASE 2026), arXiv 2603.22114

- **URL:** <https://arxiv.org/abs/2603.22114>
- **Fetched:** 2026-11-22
- **Original:** 2026-03-23 (v1); v2 2026-07-27
- **Type:** primary (author abstract + arXiv metadata)
- **Authors:** Huan Zhao, Haoxin Tu, Zhengyao Liu, Martin C. Rinard, Abhik Roychoudhury
- **Venue metadata on page:** "13 pages. To appear in ASE 26"; DOI `10.1145/3832783.3834356`

## Exact quotes

> In this paper, we argue that VC proving for program verification is more than a purely
> mathematical task, and benefits considerably from program comprehension. Our key insight is
> that human proof engineers often discover and apply helper lemmas based on their understanding
> of the program semantics, which are not directly reflected in the VCs produced by VC
> generators. Inspired by this insight, we propose an LLM agent, LemmaNet, that discovers helper
> lemmas in two ways.

> Specifically, the agent first synthesizes lemmas offline by directly analyzing the source code
> and specifications and then relating this semantic understanding to the mechanical, verbose
> encoding produced by VC generators. As the proof unfolds, LemmaNet then adapts existing helper
> lemmas online to accommodate evolving proof states, enabling the agent to effectively discharge
> complex VCs on-the-fly.

> We implement LemmaNet on top of an existing proof agent AutoRocq for Rocq and the Frama-C
> ecosystem, and evaluate it on SV-COMP and established real-world subjects, including modules of
> the Linux kernel, Contiki OS, standard C++ library, and X.509 parser.

> Our experimental results demonstrate that LemmaNet significantly outperforms state-of-the-art
> approaches, highlighting the importance of program comprehension-aided lemma discovery in
> agentic program verification.

## What it establishes

- LemmaNet exists, is public, and is accepted to ASE 2026 (per the abstract page's comment and
  DOI).
- It is a **helper-lemma discovery** agent on top of AutoRocq/Frama-C for Rocq, using offline
  source-and-spec analysis plus online adaptation to proof state.
- It occupies the "lemma synthesis conditioned on proof state" space that N2 and N9 would
  otherwise claim.

## What it does not establish

- Any specific number. The abstract says "significantly outperforms" without figures; the
  numbers are in the paper body, which I did not read.
- Anything about machine code, HOL Light, or object-code specifications.
- That lemma *reuse in a different proof* is measured (the metric N2 proposes).
