use super::*;

#[test]
fn child_progress_shares_the_parent_display_and_collapses_on_completion() {
    let buffer = Buffer::default();
    let parent = Reporter::with_writer("Preparing environment", buffer.clone());
    let child = parent.child("Installing Ruby");
    child
        .handle()
        .phase_started("Extracting archive", "Extracted archive");
    {
        let state = parent.progress.lock().unwrap();
        let frame = render::tree_frame(&state).join("\n");
        assert!(frame.contains("┌─ ● Preparing environment"), "{frame}");
        assert!(frame.contains("│  ├─ ● Installing Ruby"), "{frame}");
        assert!(frame.contains("│  │  ├─ ● Extracting archive"), "{frame}");
        assert_eq!(frame.matches("┌─").count(), 1, "{frame}");
    }
    child.finish_with_summary("Installed Ruby 4.0.9");
    {
        let state = parent.progress.lock().unwrap();
        let frame = render::tree_frame(&state).join("\n");
        assert!(frame.contains("│  ├─ ✓ Installed Ruby 4.0.9"), "{frame}");
        assert!(!frame.contains("Extracting archive"), "{frame}");
    }
    parent.finish_with_summary("Environment ready");
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("└─ ✓ Environment ready"), "{text}");
}

#[test]
fn nested_workers_keep_their_own_ids_and_plain_output_scope() {
    let buffer = Buffer::default();
    let parent = Reporter::with_plain_writer("environment", buffer.clone());
    let left = parent.child("Ruby");
    let right = parent.child("packages");
    std::thread::scope(|scope| {
        scope.spawn(|| {
            left.handle().worker_started(0, "extracting");
            left.handle().worker_output(0, "Ruby output");
            left.handle().worker_finished(0);
        });
        scope.spawn(|| {
            right.handle().worker_started(0, "compiling");
            right.handle().worker_output(0, "package output");
            right.handle().worker_finished(0);
        });
    });
    left.finish_with_summary("Ruby ready");
    right.finish_with_summary("Packages ready");
    parent.finish_with_summary("Environment ready");
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("[Ruby | worker: 0] Ruby output"), "{text}");
    assert!(
        text.contains("[packages | worker: 0] package output"),
        "{text}"
    );
    assert!(
        text.contains("[overall] complete: Environment ready"),
        "{text}"
    );
    assert!(
        text.contains("[overall] complete: Environment ready | completed 2 | failed 0"),
        "{text}"
    );
    assert!(!text.contains('\x1b'), "{text}");
}

#[test]
fn descendant_failure_prevents_compact_success_and_keeps_the_error() {
    let buffer = Buffer::default();
    let parent = Reporter::with_writer("environment", buffer.clone());
    let child = parent.child("Ruby");
    let grandchild = child.child("validation");
    grandchild.handle().fail("Missing standard library");
    grandchild.finish();
    child.finish();
    parent.finish_compact("Environment ready");
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("Missing standard library"), "{text}");
    assert!(text.contains("┌─ × environment"), "{text}");
    assert!(!text.contains("✓ Environment ready"), "{text}");
}

#[test]
fn parent_completion_closes_descendants_and_rejects_late_events() {
    let buffer = Buffer::default();
    let parent = Reporter::with_plain_writer("environment", buffer.clone());
    let child = parent.child("unfinished");
    let handle = child.handle();
    parent.finish();
    let before = buffer.0.lock().unwrap().clone();
    handle.event("late event", 1, 1, None);
    drop(child);
    assert_eq!(*buffer.0.lock().unwrap(), before);
    let text = String::from_utf8(before).unwrap();
    assert!(text.contains("failed"), "{text}");
    assert!(!text.contains("[overall] complete"), "{text}");
}

#[test]
fn finished_child_timers_stay_frozen_while_the_parent_continues() {
    let parent = Reporter::with_writer("environment", Buffer::default());
    let child = parent.child("Ruby");
    child.handle().phase_started("validating", "validated");
    child.finish_with_summary("Ruby ready");
    let before = {
        let state = parent.progress.lock().unwrap();
        (
            state.children[0].elapsed(),
            state.children[0].phase_elapsed(),
        )
    };
    std::thread::sleep(Duration::from_millis(20));
    let state = parent.progress.lock().unwrap();
    assert_eq!(state.children[0].elapsed(), before.0);
    assert_eq!(state.children[0].phase_elapsed(), before.1);
}

#[cfg(feature = "rb-task")]
#[test]
fn task_events_target_the_child_and_propagate_failure_to_the_parent() {
    let buffer = Buffer::default();
    let parent = Reporter::with_plain_writer("environment", buffer.clone());
    let child = parent.child("packages");
    let sink = child.task_sink();
    let mut graph = rb_task::TaskGraph::new();
    let task = graph.add("compile", []);
    sink.event(TaskEvent::Started {
        task,
        worker: 0,
        label: "compile".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Failed {
        task,
        worker: 0,
        error: "compiler failed".into(),
        elapsed: Duration::from_secs(1),
    });
    child.finish();
    parent.finish();
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("[packages | worker: 0 | task: 0] failed: compiler failed"),
        "{text}"
    );
    assert!(text.contains("[overall] failed"), "{text}");
    assert!(!text.contains("[overall] complete"), "{text}");
}

#[test]
fn nested_error_wrapping_preserves_the_complete_message() {
    let parent = Reporter::with_writer("environment", Buffer::default());
    let child = parent.child("Ruby");
    let validation = child.child("validation");
    let error = format!(
        "Missing standard library: {}",
        "a detailed diagnostic with Unicode 資料 ".repeat(8)
    );
    validation.handle().fail(&error);
    validation.finish();
    child.finish();
    let state = parent.progress.lock().unwrap();
    let body = render::body_lines(&state);
    let text = body
        .iter()
        .map(|line| {
            line.trim_start_matches(['│', ' ', '├', '└', '─', '×'])
                .replace(' ', "")
        })
        .collect::<String>();
    assert!(text.contains(&error.replace(' ', "")), "{body:?}");
}
