use rb_progress::Reporter;
use rb_task::{Executor, TaskAction, TaskContext, TaskEvents, TaskGraph};
use std::{collections::BTreeMap, sync::Arc, thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reporter = Reporter::start("Preparing afternoon tea").unwrap();
    let events = TaskEvents::default();
    events.subscribe(reporter.task_sink());
    let mut graph = TaskGraph::new();
    let kettle = graph.add("Boil water", []);
    let table = graph.add("Lay the table", []);
    let serve = graph.add("Serve tea", [kettle, table]);
    let actions = BTreeMap::from([
        (
            kettle,
            Arc::new(|ctx: TaskContext| {
                ctx.output("The kettle is warming.");
                thread::sleep(Duration::from_millis(1200));
                Ok(())
            }) as TaskAction,
        ),
        (
            table,
            Arc::new(|ctx: TaskContext| {
                ctx.progress("arranging", 0, 1, Some("cups and saucers".into()));
                ctx.output("Two places, neatly arranged.");
                thread::sleep(Duration::from_millis(600));
                Ok(())
            }) as TaskAction,
        ),
        (
            serve,
            Arc::new(|ctx: TaskContext| {
                ctx.output("Your tea awaits, sir.");
                thread::sleep(Duration::from_millis(400));
                Ok(())
            }) as TaskAction,
        ),
    ]);
    let result = Executor::new(2, events).run(graph, actions);
    reporter.finish_with_summary(if result.is_ok() {
        "Tea is served"
    } else {
        "Service interrupted"
    });
    result?;
    Ok(())
}
