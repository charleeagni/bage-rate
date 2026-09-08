# Written exceptions

One file per exception, and an exception exists only where a rule already
does. Two rules currently demand one.

**Replacement CRUD** (AGENTS.md): adding a repository, a mirrored DTO, a
hand-written resolver where a generated mutation exists, or a patch to a
generated file.

**A declared escape hatch** (`CustomOps::escape_hatch`): the pressure valve for
work no primitive expresses — many reads, branching on what was read, and a
write whose shape follows from them. `scripts/check-escape-hatches.mjs` fails
`verify` when a declared hatch has no record here, and fails again when one
Module declares more than two.

## The file

Name it `<module>--<operation>.md` for an escape hatch, or
`<subject>.md` for anything else. Four sections, all required for an escape
hatch and all worth writing for the rest:

```markdown
# <module>: <operation>

## Which primitive was insufficient

## Why

## The smallest seam taken

## Its drift-prevention test
```

## What an exception is for

It is not permission. It is the record a later reader needs to delete the
exception: which primitive was tried, what it could not say, and what test
will fail when the primitive grows enough to say it.

A second Module reaching for the same shape is the signal to design a
primitive and delete both records. Raising the cap instead is the failure this
directory exists to make visible.
