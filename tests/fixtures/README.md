# Disassembler capability fixtures (XVII.A1)

`aarch64_add.o` and `x86_add.o` are minimal relocatable ELF objects whose
`.text` decodes to an `add` (the x86 one also carries the required `endbr64`
prefix). `src/tools/disasm.rs` runs every candidate disassembler on the
matching object and accepts it only if the output decodes `add`; the name of
the executable is not treated as evidence of capability.

Regenerate (LLVM's integrated assembler; any `llvm-mc` from LLVM 14 onward
produces a usable object, so the committed bytes are the authority):

```sh
cd tests/fixtures
printf 'add x0, x0, x1\n'    | llvm-mc -triple=aarch64 -filetype=obj -o aarch64_add.o
printf 'endbr64\naddq %%rbx, %%rax\n' | llvm-mc -triple=x86_64 -filetype=obj -o x86_add.o
```

Verify:

```sh
llvm-objdump -d --no-show-raw-insn aarch64_add.o   # 0: add x0, x0, x1
llvm-objdump -d --no-show-raw-insn x86_add.o       # 0: endbr64; 4: add rax, rbx
```

The x86 fixture starts with `endbr64` because the production x86 path
(`define_trimmed`) requires that prefix; the probe looks for `add` anywhere in
the decoded output.

An x86-only GNU `objdump` fails the AArch64 fixture with
`can't disassemble for architecture UNKNOWN!`; that output is exactly what the
probe rejects. Do not replace these with an object whose `.text` decodes to more
than these instructions.
