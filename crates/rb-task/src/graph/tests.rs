use super::*;

#[test]
fn releases_independent_tasks_and_then_dependents() {
    let mut graph = TaskGraph::new();
    let resolve = graph.add("resolve", []);
    let loader = graph.add("loader", []);
    let pack = graph.add("pack", [resolve]);
    let snapshot = graph.add("snapshot", [pack, loader]);
    let mut schedule = graph.schedule().unwrap();
    assert_eq!(schedule.take_ready(), Some(resolve));
    assert_eq!(schedule.take_ready(), Some(loader));
    assert_eq!(schedule.take_ready(), None);
    schedule.complete(resolve).unwrap();
    assert_eq!(schedule.take_ready(), Some(pack));
    schedule.complete(loader).unwrap();
    assert_eq!(schedule.take_ready(), None);
    assert!(!schedule.is_complete());
    schedule.complete(pack).unwrap();
    assert_eq!(schedule.take_ready(), Some(snapshot));
    assert!(!schedule.is_complete());
    assert_eq!(schedule.complete(snapshot), Ok(true));
    assert!(schedule.is_complete());
    assert_eq!(schedule.take_ready(), None);
}

#[test]
fn rejects_cycles() {
    let mut graph = TaskGraph::new();
    let first = graph.add("first", [TaskId(1)]);
    let second = graph.add("second", [first]);
    assert_eq!(graph.schedule().unwrap_err(), PlanError::Cycle);
    assert_eq!(second.index(), 1);
}

#[test]
fn cannot_release_dependencies_by_completing_unscheduled_work() {
    let mut graph = TaskGraph::new();
    let first = graph.add("first", []);
    let second = graph.add("second", [first]);
    let mut schedule = graph.schedule().unwrap();
    assert_eq!(
        schedule.complete(first),
        Err(CompletionError::NotRunning(first))
    );
    assert_eq!(schedule.take_ready(), Some(first));
    assert_eq!(schedule.take_ready(), None);
    assert_eq!(schedule.complete(first), Ok(true));
    assert_eq!(schedule.complete(first), Ok(false));
    assert_eq!(schedule.take_ready(), Some(second));
}

#[test]
fn unknown_completion_does_not_panic_or_change_the_plan() {
    let mut graph = TaskGraph::new();
    let task = graph.add("work", []);
    let mut schedule = graph.schedule().unwrap();
    assert_eq!(
        schedule.complete(TaskId(99)),
        Err(CompletionError::UnknownTask(TaskId(99)))
    );
    assert_eq!(schedule.take_ready(), Some(task));
    assert!(!schedule.is_complete());
}
