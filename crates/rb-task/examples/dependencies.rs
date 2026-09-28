use rb_task::{ExecutionError, Executor, TaskAction, TaskContext, TaskEvents, TaskGraph};
use std::{collections::BTreeMap, sync::Arc, thread, time::Duration};

fn main() -> Result<(), ExecutionError> {
    let mut graph = TaskGraph::new();
    let prepare = graph.add("prepare", []);
    let left = graph.add("process left", [prepare]);
    let right = graph.add("process right", [prepare]);
    let finish = graph.add("finish", [left, right]);

    let actions = BTreeMap::from([
        (prepare, action("prepare")),
        (left, action("process left")),
        (right, action("process right")),
        (finish, action("finish")),
    ]);

    Executor::new(2, TaskEvents::default()).run(graph, actions)
}

fn action(label: &'static str) -> TaskAction {
    Arc::new(move |context: TaskContext| {
        println!("worker {}: starting {label}", context.worker);
        thread::sleep(Duration::from_millis(50));
        println!("worker {}: finished {label}", context.worker);
        Ok(())
    })
}
