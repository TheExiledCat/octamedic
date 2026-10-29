---
name: check-mmd-format
description: Check octamed's MMD parser and writer against octamed/format.md, the original MED/OctaMED MMD0-MMD3 file format documentation, and report what's read, written and round-tripped, and where the code disagrees with it. Use after changing the parser, writer or module structs in octamed/src/mmd, or when asked to "check the file format" or "check MMD support".
---

# Check MMD format support

`octamed/format.md` is Teijo Kinnunen's MMD documentation (revision 6, 1996). It's the
reference the code follows, not a description of the code: **never edit it**. This skill
checks the code against it and reports.

## 1. Read the sources

- `octamed/format.md`: the structures (`MMD0`, `MMD0song`, `MMD2song`, `MMD0Block`,
  `MMD1Block`, `BlockInfo`, `InstrHdr`, `MMD0exp`, `InstrExt`, `MMDInstrInfo`, ...), their field
  offsets and sizes, and which format version each part belongs to.
- `octamed/src/mmd/module.rs`: the Rust structs, their field order and `get_size`.
- `octamed/src/mmd/parser.rs` and `writer.rs`: what's actually read and written, and which
  pointers are followed or written as null.
- `octamed/src/mmd/conversion.rs` and `octamed/tests/parse_examples.rs`.
- `git diff` / `git log` on those files to see what changed recently, but check the whole format
  anyway: earlier gaps may never have been listed.

## 2. Compare

For each structure in the documentation, work out:

- **Read:** does the parser read every field, at the documented offset and size, with the
  documented signedness and meaning (e.g. counts stored as "lines - 1", lengths in words)?
- **Written:** does the writer write the same fields back in the same order and size? Fields
  written as null or zero (`mmd_dump`, `info`, `rexx`, MIDI commands, external instruments)
  count as not supported, even if the file still loads.
- **Version:** is it handled for every format version it exists in (MMD0, MMD1, MMD2, MMD3),
  and ignored where it doesn't?
- **Struct sizes:** does `get_size` match the documented size, including padding?

Check every claim against the code, not against comments. Where the documentation is unclear,
say so instead of guessing.

## 3. Verify

- `cargo test -p octamed`, and mention any failures rather than reporting around them.
- Where the comparison raises a question a file can answer (a field's real values, whether
  round-tripping changes the bytes), check it with the example modules in
  `octamed_cli/example_meds`. Write temporary files outside the repo and delete them after.

## 4. Report

Don't change code unless asked. Finish with:

- a short table per format version: structure, read, written, notes;
- concrete mismatches with the code location and the documented offset or meaning;
- unsupported parts that would lose data when a module is loaded and saved again;
- anything in `TODO.md` this confirms or makes outdated.
