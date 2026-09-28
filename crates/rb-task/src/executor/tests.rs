use super::*;
use crate::TaskEventSink;
use std::collections::BTreeSet;
use std::sync::{
    Condvar, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

#[derive(Default)]
struct Rendezvous {
    arrived: Mutex<usize>,
    changed: Condvar,
}

impl Rendezvous {
    fn wait(&self) {
        let mut arrived = self.arrived.lock().unwrap();
        *arrived += 1;
        self.changed.notify_all();
        let (arrived, _) = self
            .changed
            .wait_timeout_while(arrived, Duration::from_secs(2), |count| *count < 2)
            .unwrap();
        assert_eq!(*arrived, 2, "two actions must overlap");
    }
}

struct FinishedTimingSink(Mutex<Option<Duration>>);

impl TaskEventSink for FinishedTimingSink {
    fn event(&self, event: TaskEvent) {
        if let TaskEvent::Finished { elapsed, .. } = event {
            *self.0.lock().unwrap() = Some(elapsed);
        }
    }
}

#[test]
fn finished_task_reports_elapsed_time() {
    let mut graph = TaskGraph::new();
    let task = graph.add("timed", []);
    let events = TaskEvents::default();
    let timing = Arc::new(FinishedTimingSink(Mutex::new(None)));
    events.subscribe(timing.clone());
    let actions = BTreeMap::from([(
        task,
        Arc::new(|_| {
            thread::sleep(std::time::Duration::from_millis(2));
            Ok(())
        }) as TaskAction,
    )]);
    Executor::new(1, events).run(graph, actions).unwrap();
    assert!(timing.0.lock().unwrap().unwrap() >= Duration::from_millis(2));
}

#[test]
fn executor_runs_independent_tasks_in_parallel() {
    let mut graph = TaskGraph::new();
    let first = graph.add("first", []);
    let second = graph.add("second", []);
    let final_task = graph.add("final", [first, second]);
    let rendezvous = Arc::new(Rendezvous::default());
    let counts = Arc::new([
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    ]);
    let mut actions = BTreeMap::new();
    for task in [first, second] {
        let rendezvous = rendezvous.clone();
        let counts = counts.clone();
        actions.insert(
            task,
            Arc::new(move |context: TaskContext| {
                assert_eq!(context.task, task);
                rendezvous.wait();
                assert_eq!(counts[task.index()].fetch_add(1, Ordering::SeqCst), 0);
                Ok(())
            }) as TaskAction,
        );
    }
    let finished = counts.clone();
    actions.insert(
        final_task,
        Arc::new(move |_| {
            assert_eq!(finished[first.index()].load(Ordering::SeqCst), 1);
            assert_eq!(finished[second.index()].load(Ordering::SeqCst), 1);
            assert_eq!(
                finished[final_task.index()].fetch_add(1, Ordering::SeqCst),
                0
            );
            Ok(())
        }) as TaskAction,
    );
    Executor::new(2, TaskEvents::default())
        .run(graph, actions)
        .unwrap();
    assert!(counts.iter().all(|count| count.load(Ordering::SeqCst) == 1));
}

#[test]
fn limits_active_tasks_and_never_shares_a_busy_worker() {
    let active = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let busy = Arc::new(Mutex::new(BTreeSet::new()));
    let mut graph = TaskGraph::new();
    let mut actions = BTreeMap::new();
    for _ in 0..24 {
        let task = graph.add("work", []);
        let (active, completed, busy) = (active.clone(), completed.clone(), busy.clone());
        actions.insert(
            task,
            Arc::new(move |context: TaskContext| {
                assert!(busy.lock().unwrap().insert(context.worker));
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                assert!(count <= 2);
                assert!(context.worker < 2);
                thread::yield_now();
                active.fetch_sub(1, Ordering::SeqCst);
                assert!(busy.lock().unwrap().remove(&context.worker));
                completed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }) as TaskAction,
        );
    }
    Executor::new(2, TaskEvents::default())
        .run(graph, actions)
        .unwrap();
    assert_eq!(completed.load(Ordering::SeqCst), 24);
    assert!(busy.lock().unwrap().is_empty());
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[test]
fn releases_dependents_before_unrelated_running_tasks_finish() {
    let mut graph = TaskGraph::new();
    let slow = graph.add("waiting for dependent", []);
    let fast = graph.add("prerequisite", []);
    let dependent = graph.add("dependent", [fast]);
    let ran = Arc::new(AtomicBool::new(false));
    let marker = ran.clone();
    let (send, receive) = mpsc::channel();
    let receive = Mutex::new(receive);
    let actions = BTreeMap::from([
        (
            slow,
            Arc::new(move |_| {
                receive
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|e| e.to_string())
            }) as TaskAction,
        ),
        (fast, Arc::new(|_| Ok(())) as TaskAction),
        (
            dependent,
            Arc::new(move |_| {
                marker.store(true, Ordering::SeqCst);
                send.send(()).map_err(|e| e.to_string())
            }) as TaskAction,
        ),
    ]);
    Executor::new(2, TaskEvents::default())
        .run(graph, actions)
        .unwrap();
    assert!(ran.load(Ordering::SeqCst));
}

#[test]
fn failures_and_panics_join_running_work_and_block_dependents() {
    for panic in [false, true] {
        let mut graph = TaskGraph::new();
        let failed = graph.add("failure", []);
        let running = graph.add("running", []);
        let blocked = graph.add("blocked", [failed]);
        let rendezvous = Arc::new(Rendezvous::default());
        let done = Arc::new(AtomicBool::new(false));
        let blocked_ran = Arc::new(AtomicBool::new(false));
        let blocked_marker = blocked_ran.clone();
        let marker = done.clone();
        let other = rendezvous.clone();
        let actions = BTreeMap::from([
            (
                failed,
                Arc::new(move |_| {
                    rendezvous.wait();
                    if panic {
                        panic!("fixture");
                    }
                    Err("fixture".into())
                }) as TaskAction,
            ),
            (
                running,
                Arc::new(move |_| {
                    other.wait();
                    thread::sleep(Duration::from_millis(30));
                    marker.store(true, Ordering::SeqCst);
                    Ok(())
                }) as TaskAction,
            ),
            (
                blocked,
                Arc::new(move |_| {
                    blocked_marker.store(true, Ordering::SeqCst);
                    Ok(())
                }) as TaskAction,
            ),
        ]);
        let error = Executor::new(2, TaskEvents::default())
            .run(graph, actions)
            .unwrap_err();
        assert_eq!(
            error,
            if panic {
                ExecutionError::Panicked { task: failed }
            } else {
                ExecutionError::TaskFailed {
                    task: failed,
                    message: "fixture".into(),
                }
            }
        );
        assert!(done.load(Ordering::SeqCst));
        assert!(!blocked_ran.load(Ordering::SeqCst));
    }
}

#[test]
fn rejects_incorrect_action_ids_and_accepts_empty_graphs() {
    let mut graph = TaskGraph::new();
    graph.add("missing", []);
    let actions = BTreeMap::from([(TaskId(99), Arc::new(|_| Ok(())) as TaskAction)]);
    assert_eq!(
        Executor::new(1, TaskEvents::default()).run(graph, actions),
        Err(ExecutionError::InvalidActions)
    );
    Executor::new(0, TaskEvents::default())
        .run(TaskGraph::new(), BTreeMap::new())
        .unwrap();
}

#[test]
fn varied_dags_execute_every_task_once_after_its_dependencies() {
    for workers in [1, 2, 4, 8] {
        for seed in 0..12 {
            let mut graph = TaskGraph::new();
            let mut actions = BTreeMap::new();
            let counts = Arc::new((0..32).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>());
            for index in 0..32 {
                let deps = (0..index)
                    .filter(|dep| (dep * 7 + index * 11 + seed) % 9 == 0)
                    .map(TaskId)
                    .collect::<Vec<_>>();
                let task = graph.add(format!("work {index}"), deps.clone());
                let counts = counts.clone();
                actions.insert(
                    task,
                    Arc::new(move |context: TaskContext| {
                        assert!(context.worker < workers);
                        for dep in &deps {
                            assert_eq!(counts[dep.index()].load(Ordering::SeqCst), 1);
                        }
                        context.output("processing");
                        context.progress("work", 1, 1, None);
                        thread::yield_now();
                        assert_eq!(counts[index].fetch_add(1, Ordering::SeqCst), 0);
                        Ok(())
                    }) as TaskAction,
                );
            }
            Executor::new(workers, TaskEvents::default())
                .run(graph, actions)
                .unwrap();
            assert!(counts.iter().all(|count| count.load(Ordering::SeqCst) == 1));
        }
    }
}

#[test]
fn observer_panics_return_an_error_without_running_dependents() {
    struct PanickingSink(usize);
    impl TaskEventSink for PanickingSink {
        fn event(&self, event: TaskEvent) {
            let kind = match event {
                TaskEvent::Started { .. } => 0,
                TaskEvent::Progress { .. } => 1,
                TaskEvent::Output { .. } => 2,
                TaskEvent::Finished { .. } => 3,
                TaskEvent::Failed { .. } => 4,
            };
            assert_ne!(kind, self.0, "fixture observer panic");
        }
    }
    for kind in 0..5 {
        let mut graph = TaskGraph::new();
        let task = graph.add("work", []);
        let dependent = graph.add("dependent", [task]);
        let ran = Arc::new(AtomicUsize::new(0));
        let marker = ran.clone();
        let events = TaskEvents::default();
        events.subscribe(Arc::new(PanickingSink(kind)));
        let actions = BTreeMap::from([
            (
                task,
                Arc::new(move |context: TaskContext| {
                    context.progress("working", 0, 1, None);
                    context.output("output");
                    if kind == 4 {
                        Err("fixture action failure".into())
                    } else {
                        Ok(())
                    }
                }) as TaskAction,
            ),
            (
                dependent,
                Arc::new(move |_| {
                    marker.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }) as TaskAction,
            ),
        ]);
        assert_eq!(
            Executor::new(2, events).run(graph, actions),
            Err(ExecutionError::Panicked { task })
        );
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn invalid_graph_errors_preserve_their_source() {
    use std::error::Error;

    for (dependency, expected) in [
        (TaskId(0), crate::PlanError::Cycle),
        (
            TaskId(99),
            crate::PlanError::UnknownDependency(TaskId(0), TaskId(99)),
        ),
    ] {
        let mut graph = TaskGraph::new();
        let task = graph.add("invalid", [dependency]);
        let actions = BTreeMap::from([(
            task,
            Arc::new(|_| -> Result<(), String> { panic!("invalid graph executed") }) as TaskAction,
        )]);
        let error = Executor::new(1, TaskEvents::default())
            .run(graph, actions)
            .unwrap_err();
        assert_eq!(error.source().unwrap().to_string(), expected.to_string());
        assert_eq!(error, ExecutionError::InvalidGraph(expected));
    }
}

#[test]
fn zero_workers_still_executes_work() {
    let mut graph = TaskGraph::new();
    let task = graph.add("work", []);
    let ran = Arc::new(AtomicUsize::new(0));
    let marker = ran.clone();
    let actions = BTreeMap::from([(
        task,
        Arc::new(move |context: TaskContext| {
            assert_eq!(context.worker, 0);
            marker.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }) as TaskAction,
    )]);
    Executor::new(0, TaskEvents::default())
        .run(graph, actions)
        .unwrap();
    assert_eq!(ran.load(Ordering::SeqCst), 1);
}
