# rb-task

Dependency graphs, bounded workers, and task events using only the standard library.

```rust
use rb_task::{TaskEvents, Executor, TaskAction, TaskGraph};
use std::{collections::BTreeMap, sync::Arc};

let mut graph = TaskGraph::new();
let prepare = graph.add("prepare", []);
let finish = graph.add("finish", [prepare]);
let actions = BTreeMap::from([
    (prepare, Arc::new(|context: rb_task::TaskContext| {
        context.output("preparing input");
        Ok(())
    }) as TaskAction),
    (finish, Arc::new(|_| Ok(())) as TaskAction),
]);
Executor::new(2, TaskEvents::default()).run(graph, actions)?;
# Ok::<(), rb_task::ExecutionError>(())
```

Subscribe a `TaskEventSink` to the `TaskEvents` to receive lifecycle, progress,
output, and duration events. Callbacks run synchronously on workers and may
run concurrently; slow callbacks delay their worker, and panics fail the run.
The crate does not format output.

Graphs are fixed per run. Task IDs are graph-local indexes; callers must not mix
IDs from different graphs. Ready tasks run within
the worker limit as dependencies finish. On an observed action failure or panic,
or an observer panic, execution stops dispatching and waits for running work.
Cancellation, retries, and graph expansion belong to the caller.

Run `cargo test -p rb-task` for the example and regression suite.

Runnable examples:

- [Dependencies](examples/dependencies.rs): two tasks run independently between
  shared preparation and completion steps.
- [Reporting](examples/reporting.rs): observe lifecycle, output, and progress events.

```sh
cargo run -p rb-task --example dependencies
cargo run -p rb-task --example reporting
```
