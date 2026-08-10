# Privacy and Sharing Guide

`runtime.zero` is local-first, no-telemetry, and no-network by default. Local
does not mean nonsensitive: software inventories, package IDs, service labels,
versions, publishers, paths, transaction evidence, and process output can reveal
identity, work context, security posture, or installed products.

## Output classes

| Output | Default privacy posture | Sharing guidance |
| --- | --- | --- |
| `doctor` | Omits host/user/current-directory/environment/path values | Review before sharing; configuration digest and platform remain visible |
| `report` | Summary only; omits raw inventory/diagnostics, paths, names, labels, process output, and warnings | Preferred support starting point, but external sharing stays unauthorized |
| `apps` | Path-free, but includes software names, versions, publishers, source IDs, and product identifiers | Treat as private inventory unless deliberately reviewed |
| `scan --dry-run` | Paths replaced with stable report-local tokens | Still contains software/service metadata; review carefully |
| `scan --include-raw-paths` | Raw local paths included | Local-only; do not attach by default |
| `monitor` | Local counters and bounded process rows | Process names/resource patterns can be sensitive |
| update/uninstall plans | Path-free targets, manager/action IDs, digests, and capability/write posture | Private operational evidence; never share confirmation phrases |
| transaction/receipt state | Exact recovery evidence, output digests, executable identity, timestamps | Do not publish; use an explicitly approved private channel |

## Redaction semantics

Path redaction is report-local tokenization, not anonymization. Equal values in
one report map consistently so relationships remain useful. Tokens cannot be
joined across reports by design. Redaction does not remove names, versions,
identifiers, source status, counts, timestamps, or all inference risk.

Current path-bearing fields covered by the inventory redactor include:
process/user/machine PATH entries, discovered executables, application/package
metadata locations, and service/persistence metadata locations. A report with
any unredacted path-bearing field fails the strict support-export privacy gate.

## Software and service identifiers

Source-specific identifiers improve reconciliation but can be sensitive:
macOS bundle/receipt IDs, manager package IDs, Linux desktop IDs, Windows product codes/registry-product-key digests, and launchd/systemd/Windows service labels.

The support summary emits only counts and source statuses. `apps` intentionally
shows software identifiers for local review and should not be mistaken for a
support-safe export.

## Network behavior

Foundation diagnostics, inventory, monitor, report, store planning, uninstall
planning, completion generation, and fixture-backed update review do not require
network access.

Network reads occur only after an explicit updater live probe (or TUI `u`)
acknowledges `--allow-network-read`. A manager process may then contact its own
configured services; runtime.zero does not proxy or attest that traffic.
Network write acknowledgement is separate and required for the pre-alpha apply
lane. No command sends a support report automatically.

Website source is a static brochure/demo. It does not install software or become
an authority merely because it renders in a browser.

## Before sharing anything

1. Prefer `rz0 report --format json`.
2. Read the complete file, not only its first lines.
3. Confirm that it contains no software names, service labels, paths, usernames,
   hostnames, environment values, credentials, confirmation phrases, transaction
   files, or process output.
4. Remove unrelated terminal history and shell prompts from screenshots.
5. Use a private, explicitly approved transfer channel for any necessary raw
   evidence.
6. Set an evidence-retention/deletion expectation with the recipient.

`local_export_ready: true` means only that the strict local schema/privacy gate
passed. `external_sharing_authorized: false` is intentional and must not be
changed by an automated command.

## Public issue guidance

A useful first report contains:

- runtime.zero version or source commit;
- target OS/architecture family;
- exact command shape with private values replaced;
- exit code and a manually reviewed redacted error;
- whether state may have been written;
- the privacy-reviewed support summary when appropriate.

Never publish tokens, keys, cookies, credential files, OAuth/session state,
browser profiles, customer/employer data, private repositories, full home
paths, confirmation phrases, executable output captures, or transaction/receipt
files.

## Retention and local cleanup

No production evidence-retention command exists yet. Do not manually sweep the
state root. Recovery evidence can be necessary to distinguish no write from an
unknown external effect. A future retention feature must inventory first,
classify ownership, preserve active/recovery-required transactions, support a
dry run, and use quarantine before deletion.

Shell completion output and the manual page contain no host-specific evidence.
Generated benchmark/package evidence binds artifact and source identities and
should still be reviewed before publication.

## Threat-model caveats

- Redaction cannot prevent inference from unique package combinations or counts.
- Digests can be identifying when their input space is small; report digests are
  domain-separated but not proof of anonymity.
- A manager can reveal data through its own output or network behavior.
- The process host bounds captured bytes, but does not constitute a platform
  sandbox.
- Parsing a report does not prove who produced it or that its source was honest.
- No external privacy/security review or production incident-retention exercise
  has completed.

See [`privacy-contract.md`](privacy-contract.md),
[`support-report-contract.md`](support-report-contract.md), and
[`recovery-guide.md`](recovery-guide.md) for normative details.
