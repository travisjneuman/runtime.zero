# Transaction simulation fixture

`workspace/stale-shim.bin` is synthetic non-executable data. Integration tests
copy it to a marked direct OS-temp child, then exercise verified quarantine,
partial failure, conflict refusal, and restore behavior. No repository fixture
or production path is mutated.
