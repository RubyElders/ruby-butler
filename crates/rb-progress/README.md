# rb-progress

Progress reporting with a live terminal tree and plain, append-only output when
stderr is redirected. Supports parallel workers, elapsed times, and errors.

```rust
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

Nested operations share the parent display and propagate failures:

```rust
use rb_progress::Reporter;

let parent = Reporter::start("Preparing the room").unwrap();
let child = parent.child("Polishing the silver");
child.handle().event("polishing", 3, 3, None);
child.finish_with_summary("Silver polished");
parent.finish_with_summary("The room is ready");
```

Completed children collapse to their summary:

```text
┌─ ✓ Preparing the room
│  ├─ ✓ Silver polished (0.0s)
└─ ✓ The room is ready (0.0s)
```

`rb-task` is optional and disabled by default. Enable the `rb-task` feature to
connect `Reporter::task_sink()` to `TaskEvents`.

Run the examples:

```sh
cargo run -p rb-progress --example standalone
cargo run -p rb-progress --example parallel
cargo run -p rb-progress --example failure
cargo run -p rb-progress --features rb-task --example tasks
```

Use `handle.byte_progress("downloading", received, total)` for transfer sizes. TTY and plain output share human-readable units and percentages.
