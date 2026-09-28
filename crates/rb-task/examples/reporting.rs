use rb_task::{
    ExecutionError, Executor, TaskAction, TaskContext, TaskEvent, TaskEventSink, TaskEvents,
    TaskGraph,
};
use std::{collections::BTreeMap, sync::Arc};

struct Console;

impl TaskEventSink for Console {
    fn event(&self, event: TaskEvent) {
        match event {
            TaskEvent::Started { worker, label, .. } => {
                println!("worker {worker}: {label}");
            }
            TaskEvent::Output { line, .. } => println!("  {line}"),
            TaskEvent::Progress {
                phase, done, total, ..
            } => {
                println!("  {phase}: {done}/{total}");
            }
            TaskEvent::Finished { elapsed, .. } => println!("  finished in {elapsed:?}"),
            TaskEvent::Failed { error, .. } => println!("  failed: {error}"),
        }
    }
}

fn main() -> Result<(), ExecutionError> {
    let events = TaskEvents::default();
    events.subscribe(Arc::new(Console));

    let mut graph = TaskGraph::new();
    let process = graph.add("process documents", []);
    let actions = BTreeMap::from([(
        process,
        Arc::new(|context: TaskContext| {
            for done in 1..=3 {
                context.output(format!("Processed document {done}"));
                context.progress("processing", done, 3, None);
            }
            Ok(())
        }) as TaskAction,
    )]);

    Executor::new(1, events).run(graph, actions)
}
