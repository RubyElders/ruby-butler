# rb-progress

Progress reporting with a live terminal tree and plain, append-only output when
stderr is redirected. Supports parallel workers, elapsed times, and errors.

```rust,no_run
use rb_progress::Reporter;

let reporter = Reporter::start("Polishing the silver").unwrap();
let progress = reporter.handle();
progress.event("polishing", 0, 3, None);
// Do the work, then report completion.
progress.event("polishing", 3, 3, None);
reporter.finish_with_summary("The silver is ready, sir");
```

Terminal output (timing varies):

```text
┌─ ✓ Polishing the silver
│  ├─ ✓ polishing 3/3 (0.0s)
└─ ✓ The silver is ready, sir (0.0s)
```

Clone a `ProgressHandle` to report from multiple threads. Use `worker_started`,
`worker_event`, and `worker_finished` or `worker_failed` for each worker.
Use `fail` to report an overall error.

`rb-task` is optional and disabled by default. Enable the `rb-task` feature to
connect `Reporter::task_sink()` to `TaskEvents`.

Run the examples:

```sh
cargo run -p rb-progress --example standalone
cargo run -p rb-progress --example parallel
cargo run -p rb-progress --example failure
cargo run -p rb-progress --features rb-task --example tasks
```
