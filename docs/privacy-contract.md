# Privacy and Redaction Contract

`crates/privacy-contract/` owns cross-module privacy classification and report-
local redaction. Modules must not create private placeholder grammars or retain
extra copies of raw sensitive values merely to redact output.

Schema 1 classifies local paths, environment values, registry locations, process
output, and command arguments as requiring report-local redaction. User and host
identities are not collected by default.

`RedactionContext` is bounded to 9,999 unique tokens. It stores only a domain-
separated SHA-256 key and sequence number, not the original string. Canonically
ordered traversal produces stable placeholders such as
`<redacted:path:0001>`. Equal values within one class reuse a token; classes are
domain-separated. Empty values, zero/expanded ceilings, and ceiling exhaustion
fail closed.

The inventory module now uses this foundation context and redacts paths by
default. `--include-raw-paths` is an explicit local-only disclosure choice; it
never changes the production protocol requirement that paths be redacted.
Redaction is privacy reduction, not anonymization: application names and other
opt-in evidence still require review before sharing.
