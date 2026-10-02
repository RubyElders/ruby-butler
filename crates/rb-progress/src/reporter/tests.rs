use super::*;
#[cfg(feature = "rb-task")]
use rb_task::TaskEvent;

#[test]
fn completed_phases_remain_as_checked_history_rows() {
    let mut progress = ProgressState {
        title: "demo".into(),
        ..Default::default()
    };
    progress.show(ProgressEvent {
        worker: None,
        phase: "resolving",
        done: 1,
        total: 1,
        detail: None,
        elapsed: None,
    });
    progress.show(ProgressEvent {
        worker: None,
        phase: "downloading",
        done: 0,
        total: 2,
        detail: None,
        elapsed: None,
    });
    let rows = render::history_lines(&progress);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].contains("✓ resolving"));
    assert!(rows[0].contains("1/1"));
}

#[test]
fn native_handoff_is_rendered_once_until_completion() {
    let mut progress = ProgressState {
        title: "demo".into(),
        phase: "preparing native extensions",
        ..Default::default()
    };
    progress.external_output = true;
    render::draw(&mut progress);
    assert!(progress.handoff_drawn);
    let lines = progress.rendered_lines;
    render::draw(&mut progress);
    assert_eq!(progress.rendered_lines, lines);
}

#[test]
fn fetch_progress_updates_keep_separate_phase_rows() {
    let mut progress = ProgressState {
        title: "demo".into(),
        ..Default::default()
    };
    for phase in ["downloading", "unpacking", "downloading", "unpacking"] {
        progress.show(ProgressEvent {
            worker: None,
            phase,
            done: 1,
            total: 2,
            detail: None,
            elapsed: None,
        });
    }
    assert_eq!(progress.history.len(), 2);
    assert_eq!(progress.history[0].phase, "downloading");
    assert_eq!(progress.history[1].phase, "unpacking");
    assert_eq!(progress.phase, "unpacking");
    assert_eq!(render::history_lines(&progress).len(), 2);
    assert!(render::history_lines(&progress)[0].contains("downloading"));
}

#[test]
fn empty_reporter_does_not_claim_prompt_rows() {
    let mut progress = ProgressState::default();
    render::draw(&mut progress);
    assert_eq!(progress.rendered_lines, 0);
    assert!(!progress.anchor_started);
}

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Buffer {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

#[test]
fn reporters_have_independent_sinks_and_ignore_events_after_finish() {
    let first = Buffer::default();
    let second = Buffer::default();
    let a = Reporter::with_writer("Building documents", first.clone());
    let b = Reporter::with_writer("Copying images", second.clone());
    let old = a.handle();
    old.event("preparing content", 0, 3, None);
    assert!(!a.progress.lock().unwrap().external_output);
    b.handle().event("copying", 1, 2, None);
    a.finish_with_summary("Documents ready");
    let finished = first.text();
    old.event("late event", 1, 1, None);
    assert_eq!(first.text(), finished);
    b.handle().event("copying", 2, 2, None);
    b.finish_with_summary("Images ready");
    assert!(finished.contains("Building documents") && finished.contains("Documents ready"));
    assert!(!finished.contains("Copying images") && !finished.contains("bundle"));
    assert!(
        second.text().contains("Images ready") && !second.text().contains("Building documents")
    );
}

#[cfg(feature = "rb-task")]
#[test]
fn task_output_stays_on_its_worker_and_completion_freezes_the_clock() {
    let reporter = Reporter::with_writer("Processing files", Buffer::default());
    let sink = reporter.task_sink();
    let mut graph = rb_task::TaskGraph::new();
    let task = graph.add("document", []);
    sink.event(TaskEvent::Started {
        task,
        worker: 0,
        label: "document".into(),
        elapsed: Duration::ZERO,
    });
    for line in ["one", "two", "three", "four"] {
        sink.event(TaskEvent::Output {
            task,
            worker: 0,
            line: line.into(),
            elapsed: Duration::ZERO,
        });
    }
    sink.event(TaskEvent::Finished {
        task,
        worker: 0,
        elapsed: Duration::from_secs(12),
    });
    let state = reporter.progress.lock().unwrap();
    assert!(state.phase.is_empty());
    assert_eq!(
        state.workers[&0]
            .output
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["two", "three", "four"]
    );
    let before = render::body_lines(&state)
        .into_iter()
        .find(|line| line.contains("worker 0"));
    drop(state);
    reporter
        .progress
        .lock()
        .unwrap()
        .workers
        .get_mut(&0)
        .unwrap()
        .started -= Duration::from_secs(2);
    assert_eq!(
        render::body_lines(&reporter.progress.lock().unwrap())
            .into_iter()
            .find(|line| line.contains("worker 0")),
        before
    );
    reporter.finish();
}

#[cfg(feature = "rb-task")]
#[test]
fn failed_task_is_not_presented_as_successful_completion() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("processing documents", buffer.clone());
    let mut graph = rb_task::TaskGraph::new();
    let task = graph.add("export", []);
    reporter.task_sink().event(TaskEvent::Failed {
        task,
        worker: 0,
        error: "fixture".into(),
        elapsed: Duration::from_secs(1),
    });
    reporter.finish();
    let output = buffer.text();
    assert!(output.contains("┌─ × processing documents"));
    assert!(output.contains("└─ × failed"));
    assert!(!output.contains("└─ ✓ complete"));
}

#[cfg(feature = "rb-task")]
#[test]
fn worker_reuse_keeps_failures_and_clears_only_the_previous_task_output() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("documents", buffer.clone());
    let sink = reporter.task_sink();
    let mut graph = rb_task::TaskGraph::new();
    let first = graph.add("first", []);
    let second = graph.add("second", []);
    sink.event(TaskEvent::Started {
        task: first,
        worker: 0,
        label: "first".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Output {
        task: first,
        worker: 0,
        line: "one\ntwo\nthree\nfour".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Progress {
        task: first,
        worker: 0,
        phase: "running",
        done: 0,
        total: 1,
        detail: None,
        elapsed: Duration::ZERO,
    });
    let output = reporter.progress.lock().unwrap().workers[&0].output.clone();
    assert_eq!(
        output.into_iter().collect::<Vec<_>>(),
        ["two", "three", "four"]
    );
    sink.event(TaskEvent::Failed {
        task: first,
        worker: 0,
        error: "fixture".into(),
        elapsed: Duration::from_secs(1),
    });
    sink.event(TaskEvent::Started {
        task: second,
        worker: 0,
        label: "second".into(),
        elapsed: Duration::ZERO,
    });
    let empty = reporter.progress.lock().unwrap().workers[&0]
        .output
        .is_empty();
    assert!(empty);
    sink.event(TaskEvent::Finished {
        task: second,
        worker: 0,
        elapsed: Duration::from_secs(2),
    });
    let frame = render::tree_frame(&reporter.progress.lock().unwrap());
    assert!(
        frame
            .last()
            .unwrap()
            .contains("completed 1 | active 0 | failed 1")
    );
    reporter.finish();
    assert!(buffer.text().contains("└─ × failed"));
}

#[cfg(feature = "rb-task")]
#[test]
fn phase_names_do_not_stop_running_timers() {
    let reporter = Reporter::with_writer("documents", Vec::new());
    let mut graph = rb_task::TaskGraph::new();
    let task = graph.add("work", []);
    for phase in ["done", "failed", "running"] {
        reporter.task_sink().event(TaskEvent::Progress {
            task,
            worker: 0,
            phase,
            done: 0,
            total: 1,
            detail: None,
            elapsed: Duration::from_secs(1),
        });
        let mut state = reporter.progress.lock().unwrap();
        state.workers.get_mut(&0).unwrap().started -= Duration::from_secs(2);
        let lines = render::body_lines(&state);
        drop(state);
        assert!(lines.iter().any(|line| line.contains("(3.0s)")));
    }
    reporter.finish();
}

#[cfg(feature = "rb-task")]
#[test]
fn plain_output_prefixes_every_line_and_keeps_errors_after_worker_reuse() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_plain_writer("china", buffer.clone());
    assert!(reporter.ticker.is_none());
    let sink = reporter.task_sink();
    let mut graph = rb_task::TaskGraph::new();
    let first = graph.add("inspect", []);
    let second = graph.add("replace", []);
    sink.event(TaskEvent::Started {
        task: first,
        worker: 0,
        label: "inspect".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Output {
        task: first,
        worker: 0,
        line: "one\ntwo\x1b[2J".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Failed {
        task: first,
        worker: 0,
        error: "Replace the teacup".into(),
        elapsed: Duration::from_secs(1),
    });
    sink.event(TaskEvent::Started {
        task: second,
        worker: 0,
        label: "replace".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Finished {
        task: second,
        worker: 0,
        elapsed: Duration::from_secs(2),
    });
    reporter.finish();
    let text = buffer.text();
    assert!(!text.contains('\x1b'));
    assert!(!text.contains('\r'));
    assert!(text.contains("[worker: 0 | task: 0] one\n[worker: 0 | task: 0] two\n"));
    assert!(text.contains("[worker: 0 | task: 0] failed: Replace the teacup (1.0s)"));
    assert!(text.contains("[overall] failed | completed 1 | failed 1"));
    sink.event(TaskEvent::Output {
        task: second,
        worker: 0,
        line: "late".into(),
        elapsed: Duration::ZERO,
    });
    assert_eq!(buffer.text(), text);
}

#[cfg(feature = "rb-task")]
#[test]
fn terminal_finish_retains_the_error_after_worker_reuse() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("china", buffer.clone());
    let sink = reporter.task_sink();
    let mut graph = rb_task::TaskGraph::new();
    let first = graph.add("inspect", []);
    let second = graph.add("replace", []);
    sink.event(TaskEvent::Failed {
        task: first,
        worker: 0,
        error: "Replace the teacup".into(),
        elapsed: Duration::from_secs(1),
    });
    sink.event(TaskEvent::Started {
        task: second,
        worker: 0,
        label: "replace".into(),
        elapsed: Duration::ZERO,
    });
    sink.event(TaskEvent::Finished {
        task: second,
        worker: 0,
        elapsed: Duration::from_secs(2),
    });
    buffer.0.lock().unwrap().clear();
    reporter.finish();
    assert!(buffer.text().contains("Replace the teacup"));
}

#[test]
fn live_summaries_include_an_activity_icon_for_both_reporting_styles() {
    let mut state = ProgressState::default();
    for phase in ["", "polishing"] {
        state.phase = phase;
        let frame = render::tree_frame(&state);
        assert!(frame.last().unwrap().starts_with("└─ ⠋ "));
    }
}

#[test]
fn standalone_plain_progress_needs_no_executor() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_plain_writer("silver", buffer.clone());
    let handle = reporter.handle();
    handle.event("polishing", 1, 3, None);
    handle.worker_event(Some(2), "polishing", 2, 3, Some("spoons".into()));
    reporter.finish_with_summary("silver ready");
    let output = buffer.text();
    assert!(output.contains("[overall] polishing 1/3"));
    assert!(output.contains("[worker: 2] polishing 2/3 spoons"));
    assert!(output.contains("[overall] complete: silver ready"));
    assert!(!output.contains('\x1b'));
    handle.event("late", 3, 3, None);
    assert_eq!(buffer.text(), output);
}

#[test]
fn standalone_failure_survives_later_progress_and_finish() {
    for plain in [false, true] {
        let buffer = Buffer::default();
        let reporter = if plain {
            Reporter::with_plain_writer("china", buffer.clone())
        } else {
            Reporter::with_writer("china", buffer.clone())
        };
        let handle = reporter.handle();
        handle.fail("Replace the teacup");
        handle.event("cleanup", 1, 1, None);
        if !plain {
            buffer.0.lock().unwrap().clear();
        }
        reporter.finish();
        let output = buffer.text();
        assert!(output.contains("Replace the teacup"));
        assert!(output.contains(if plain {
            "[overall] failed"
        } else {
            "└─ × failed"
        }));
        handle.fail("late failure");
        assert_eq!(buffer.text(), output);
    }
}

#[test]
fn standalone_workers_complete_once_freeze_timers_and_can_be_reused() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_plain_writer("dining room", buffer.clone());
    let handle = reporter.handle();
    handle.worker_started(0, "silver");
    handle.worker_event(Some(0), "polishing", 3, 3, None);
    assert_eq!(reporter.progress.lock().unwrap().completed_tasks, 0);
    handle.worker_finished(0);
    handle.worker_finished(0);
    handle.worker_failed(0, "late failure");
    let mut state = reporter.progress.lock().unwrap();
    assert_eq!(state.completed_tasks, 1);
    assert_eq!(state.failed_tasks, 0);
    let before = render::body_lines(&state);
    state.workers.get_mut(&0).unwrap().started -= Duration::from_secs(10);
    assert_eq!(render::body_lines(&state), before);
    drop(state);
    handle.worker_started(0, "china");
    handle.worker_failed(0, "Replace the teacup");
    handle.worker_failed(0, "duplicate");
    handle.worker_started(0, "napkins");
    handle.worker_finished(0);
    reporter.finish();
    let text = buffer.text();
    assert!(text.contains("[worker: 0] failed: Replace the teacup"));
    assert!(text.contains("[overall] failed | completed 2 | failed 1"));
    handle.worker_started(1, "late");
    handle.worker_finished(1);
    assert_eq!(buffer.text(), text);
}

#[test]
fn standalone_threads_share_workers_and_completion_counts() {
    use std::sync::Barrier;
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("dining room", buffer.clone());
    let handle = reporter.handle();
    let ready = Barrier::new(4);
    let release = Barrier::new(4);
    thread::scope(|scope| {
        for worker in 0..3 {
            let handle = handle.clone();
            let ready = &ready;
            let release = &release;
            scope.spawn(move || {
                handle.worker_started(worker, "polishing");
                ready.wait();
                release.wait();
                handle.worker_finished(worker);
            });
        }
        ready.wait();
        let state = reporter.progress.lock().unwrap();
        let worker_count = state.workers.len();
        let frame = render::tree_frame(&state).join("\n");
        drop(state);
        release.wait();
        assert_eq!(worker_count, 3);
        assert!(frame.contains("active 3"));
        for worker in 0..3 {
            assert!(frame.contains(&format!("worker {worker}")));
        }
    });
    let state = reporter.progress.lock().unwrap();
    let frame = render::tree_frame(&state).join("\n");
    assert!(frame.contains("completed 3 | active 0 | failed 0"));
    drop(state);
    reporter.finish();
}
