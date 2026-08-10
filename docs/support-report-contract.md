# Privacy-Reviewed Support Report Contract

`crates/support-contract/` owns the deterministic schema-1 support summary used
by report/export modules. Modules select inputs and output format; they do not
create private validation, hashing, privacy, or authority policies.

## Inputs and construction

The foundation also owns the bounded strict `support_report_input` envelope and
its decoder. `build_support_report` accepts the envelope's one strict
`inventory_report` and one strict `foundation_diagnostics` value. It requires:

- both shared validators to pass;
- inventory to pass its separate `private_for_export` gate;
- exact OS/architecture agreement;
- no host/user identity or raw registry keys;
- redacted canonical tokens for every path-bearing inventory field;
- canonical diagnostics configuration binding.

The builder hashes canonical serialized inputs with separate domain strings. It
never embeds either raw input. The deterministic report contains only:

- OS and architecture class;
- canonical configuration SHA-256;
- domain-separated inventory/diagnostics digests;
- inventory counts and bounded source IDs/statuses;
- diagnostics status counts;
- exact privacy and non-authority posture.

Application names, local paths, environment values, process output, diagnostic
details, events, and free-form warnings are omitted. Output is bounded to 64
KiB, strict-deserialized with unknown-field rejection, and validated again
before text or JSON rendering.

## Authority boundary

A valid report sets `local_export_ready: true`: the bytes may be emitted to the
local caller. It always sets these fields to false:

- `external_sharing_authorized`;
- `product_execution_authorized`;
- `release_authorized`.

The report does not transmit data, write files, approve disclosure, authorize a
module, or satisfy release acceptance. Input digests are evidence bindings, not
signatures or trust roots.

## Foundation and first-party surfaces

The installed foundation exposes a local read-only construction path:

```bash
rz0 report
rz0 report --format json
```

It collects redacted live inventory plus private diagnostics in memory, builds
the same strict summary, and writes only rendered summary bytes to stdout. It
accepts no network/output path and does not invoke the separate module binary.

`modules/report-export/` remains a separate source package and development binary:

```bash
cargo run -p rz0-module-report-export -- --format json < report-export-input.json
```

The input envelope is bounded before parsing and arrives only on standard input;
the module accepts no input/output path or network option. Output goes only to
standard output. Its manifest declares no host capability because it consumes
caller-supplied framed data rather than reading host state. The core does not
install, load, or execute this module.

The module family remains incomplete for 1.0 until signed artifact lifecycle,
final-artifact Windows/macOS/Linux runtime evidence,
support-bundle attachments where explicitly designed, accessibility, and every
canonical release cell pass. A source-level read-only exporter is not a release
claim.
