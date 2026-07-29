# Immutable staging fixtures

`input/package/` is a synthetic, non-executable package. `valid-plan.json` binds
its manifest and payload digests to a test-key signature verification and to
simulation-only source/staging/publication roots.

Integration tests copy these files into a marked direct child of the OS temp
root. No repository fixture is modified, installed, activated, or executed.
