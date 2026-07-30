# Report and export module

`rz0-module-report-export` is the first-party schema-1 report/export source
package. It contains only domain behavior: select a privacy-reviewed summary
from already validated inventory and diagnostics inputs and render text or JSON.
The foundation crate `rz0-support-contract` owns validation, canonical digests,
privacy posture, authority refusal, output shape, and rendering.

The module is not installed, loaded, or executed by `rz0`. The foundation owns
the strict `support_report_input` envelope and its byte ceiling. Development use is
stdin/stdout only:

```bash
cargo run -p rz0-module-report-export -- --format json < report-export-input.json
```

Input is one strict `support_report_input` JSON object containing an
`inventory_report` and `foundation_diagnostics`. It is bounded before parsing.
Unknown fields, raw-path inventory, identity-bearing inventory, platform drift,
summary drift, malformed cross-references, and invalid diagnostics fail closed.

Output deliberately excludes raw reports, local paths, identities, environment
values, application names, process output, and free-form warnings. It includes
only counts, source IDs/statuses, platform class, configuration digest, and
domain-separated input digests. `local_export_ready: true` means bytes are safe
to emit locally; `external_sharing_authorized`, `product_execution_authorized`,
and `release_authorized` remain false. No output is transmitted or written to a
file by the module.
