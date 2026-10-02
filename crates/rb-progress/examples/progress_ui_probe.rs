use rb_progress::Reporter;
use std::{thread, time::Duration};

fn main() {
    if std::env::args().any(|arg| arg == "--aggregate") {
        let reporter = Reporter::start("processing documents").unwrap();
        reporter.handle().event("reading", 1, 1, None);
        reporter.handle().event("writing", 1, 1, None);
        reporter.finish_with_summary("documents ready");
        return;
    }
    if std::env::args().any(|arg| matches!(arg.as_str(), "--tasks" | "--stress" | "--failure")) {
        task_probe();
        return;
    }
    let reporter = Reporter::start("preparing pty-probe bundle").unwrap();
    let progress = reporter.handle();
    thread::sleep(Duration::from_millis(350));
    progress.event("resolved", 1, 1, None);
    for worker in 0..12 {
        progress.event("downloading", worker, 12, None);
        progress.worker_event(
            Some(worker),
            "downloading",
            worker,
            12,
            Some(format!("gem-{worker}")),
        );
        progress.event("unpacking", worker + 1, 12, None);
    }
    thread::sleep(Duration::from_millis(300));
    progress.event("packing LMDB", 0, 1, Some("records".into()));
    progress.worker_event(Some(9), "packing LMDB", 1, 1, Some("records".into()));
    progress.event("packing LMDB", 1, 1, Some("records".into()));
    progress.event("preparing native extensions", 0, 0, None);
    if std::env::args().any(|arg| arg == "--handoff-output") {
        let mut graph = rb_task::TaskGraph::new();
        let task = graph.add("preparing content", []);
        for line in ["preview one", "preview two", "preview three"] {
            reporter.task_sink().event(rb_task::TaskEvent::Output {
                task,
                worker: 11,
                line: line.into(),
                elapsed: Duration::ZERO,
            });
        }
    }
    progress.suspend();
    let native_lines = 10;
    eprint!("\x1b[1B\r\x1b[{}L\x1b[1A\r", native_lines - 1);
    let rows = [
        "├─ ● installing pty-probe bundle | compiling native extensions",
        "│  ├─ ● worker 0 (compiling) demo-1.0",
        "│     checking for ruby.h... yes",
        "│  └─ ● worker 1 (compiling) demo-1.0",
        "│     compiling demo.c",
        "└─ overall: 1/2 native tasks | pending 1 | elapsed 0.3s",
    ];
    for index in 0..native_lines {
        let line = rows.get(index).copied().unwrap_or("");
        eprint!("\x1b[2K{line}");
        if index + 1 < native_lines {
            eprintln!();
        }
    }
    eprint!(
        "\x1b[{}A\r\x1b[0J\x1b[{}M",
        native_lines - 1,
        native_lines - 1
    );
    progress.event("preparing", 1, 1, None);
    thread::sleep(Duration::from_millis(50));
    reporter.finish_with_summary("your meticulously prepared bundle is ready, sir (.rb)");
    println!("probe complete");
}

fn task_probe() {
    use rb_task::{Executor, TaskAction, TaskEvents, TaskGraph};
    use std::{collections::BTreeMap, sync::Arc};

    let stress = std::env::args().any(|arg| arg == "--stress");
    let fail = std::env::args().any(|arg| arg == "--failure");
    let reporter = Reporter::start("processing documents").unwrap();
    let events = TaskEvents::default();
    events.subscribe(reporter.task_sink());
    let mut graph = TaskGraph::new();
    let prepare = graph.add("preparing content", []);
    let publish = graph.add("exporting documents", [prepare]);
    let mut actions = BTreeMap::from([
        (
            prepare,
            Arc::new(|context: rb_task::TaskContext| {
                context.progress("preparing content", 0, 1, None);
                context.output("reading document");
                thread::sleep(Duration::from_millis(300));
                Ok(())
            }) as TaskAction,
        ),
        (
            publish,
            Arc::new(|context: rb_task::TaskContext| {
                context.progress("exporting documents", 0, 1, None);
                context.output("writing document");
                thread::sleep(Duration::from_millis(300));
                Ok(())
            }) as TaskAction,
        ),
    ]);
    if stress || fail {
        for index in 0..16 {
            let task = graph.add(format!("document {index}"), []);
            actions.insert(
                task,
                Arc::new(move |context: rb_task::TaskContext| {
                    context.progress("failed", 0, 1, None);
                    context.output(format!(
                        "\x1b[2J\x1b[31m{}\x1b[0m\nworking on document {index}\npreview e\u{301}",
                        "資料".repeat(100)
                    ));
                    thread::sleep(Duration::from_millis(30));
                    if fail && index == 0 {
                        Err("document failure".into())
                    } else {
                        Ok(())
                    }
                }) as TaskAction,
            );
        }
    }
    let result = Executor::new(if stress || fail { 6 } else { 2 }, events).run(graph, actions);
    assert_eq!(result.is_err(), fail);
    reporter.finish_with_summary(if fail {
        "processing failed"
    } else {
        "documents ready"
    });
    println!("probe complete");
}
