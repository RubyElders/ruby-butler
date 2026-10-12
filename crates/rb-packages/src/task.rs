use anyhow::{Result, anyhow};
use rb_task::{Executor, TaskAction, TaskContext, TaskEvents, TaskGraph};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub(crate) fn run<T: Send + 'static>(
    label: &str,
    events: TaskEvents,
    work: impl Fn(TaskContext) -> Result<T> + Send + Sync + 'static,
) -> Result<T> {
    let output = Arc::new(Mutex::new(None));
    let result = output.clone();
    let mut graph = TaskGraph::new();
    let id = graph.add(label, []);
    let action: TaskAction = Arc::new(move |context| {
        *result.lock().unwrap() = Some(work(context).map_err(|e| format!("{e:#}"))?);
        Ok(())
    });
    Executor::new(1, events).run(graph, BTreeMap::from([(id, action)]))?;
    output
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| anyhow!("Task did not return a result"))
}
