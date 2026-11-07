use super::*;

#[test]
fn byte_progress_keeps_human_units_in_live_plain_and_final_output() {
    for plain in [false, true] {
        let buffer = Buffer::default();
        let reporter = if plain {
            Reporter::with_plain_writer("download", buffer.clone())
        } else {
            Reporter::with_writer("download", buffer.clone())
        };
        let handle = reporter.handle();
        handle.byte_progress("downloading", 12 * 1024 * 1024, 48 * 1024 * 1024);
        if !plain {
            let state = reporter.progress.lock().unwrap();
            let frame = render::tree_frame(&state);
            assert!(
                frame
                    .iter()
                    .any(|line| line.starts_with("│  ├─ ● downloading 12.0 MiB / 48.0 MiB (25%)"))
            );
        }
        handle.byte_progress("downloading", 48 * 1024 * 1024, 48 * 1024 * 1024);
        handle.event("validating", 1, 1, None);
        if !plain {
            buffer.0.lock().unwrap().clear();
        }
        reporter.finish_with_summary("ready");
        let output = buffer.text();
        assert!(
            output.contains("downloading 48.0 MiB / 48.0 MiB (100%)"),
            "{output}"
        );
        assert!(!output.contains("50331648"));
    }
}

#[test]
fn phase_errors_are_children_of_the_failed_phase() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("installing", buffer.clone());
    let handle = reporter.handle();
    handle.event("extracting", 1, 1, None);
    handle.event("validating", 0, 1, None);
    handle.fail("cannot load bundled gem");
    buffer.0.lock().unwrap().clear();
    reporter.finish();
    let output = buffer.text();
    assert!(output.contains("│  ├─ ✓ extracting"), "{output}");
    assert!(output.contains("│  ├─ × validating"), "{output}");
    assert!(
        output.contains("│  │  └─ × cannot load bundled gem"),
        "{output}"
    );
    assert_eq!(output.matches("cannot load bundled gem").count(), 1);
}

#[test]
fn download_is_reported_once_with_overall_elapsed_time_in_the_footer() {
    let mut state = ProgressState::default();
    state.show_with_unit(
        ProgressEvent {
            worker: None,
            phase: "downloading Ruby",
            done: 12 * 1024 * 1024,
            total: 48 * 1024 * 1024,
            detail: None,
            elapsed: None,
        },
        crate::progress_unit::ProgressUnit::Bytes,
    );
    let frame = render::tree_frame(&state);
    assert_eq!(
        frame
            .iter()
            .filter(|line| line.contains("downloading Ruby"))
            .count(),
        1,
        "{frame:?}"
    );
    assert_eq!(
        frame
            .iter()
            .filter(|line| line.contains("12.0 MiB / 48.0 MiB (25%)"))
            .count(),
        1,
        "{frame:?}"
    );
    assert!(frame.last().unwrap().starts_with("└─ "));
    assert!(frame.last().unwrap().contains("elapsed "));
}

#[test]
fn repeated_phase_updates_have_one_visible_row_per_phase() {
    let mut state = ProgressState::default();
    for phase in ["downloading", "unpacking", "downloading", "unpacking"] {
        state.show(ProgressEvent {
            worker: None,
            phase,
            done: 1,
            total: 2,
            detail: Some("fixture".into()),
            elapsed: None,
        });
    }
    let frame = render::tree_frame(&state);
    for phase in ["downloading", "unpacking"] {
        assert_eq!(
            frame.iter().filter(|line| line.contains(phase)).count(),
            1,
            "{frame:?}"
        );
    }
}

#[test]
fn revisited_phase_keeps_its_accumulated_time() {
    let mut state = ProgressState::default();
    for phase in ["downloading", "unpacking", "downloading"] {
        state.show(ProgressEvent {
            worker: None,
            phase,
            done: 1,
            total: 2,
            detail: None,
            elapsed: None,
        });
    }
    state
        .history
        .iter_mut()
        .find(|record| record.phase == "downloading")
        .unwrap()
        .elapsed = Duration::from_secs(2);
    state.phase_started = None;
    assert_eq!(state.phase_elapsed(), Duration::from_secs(2));
    let frame = render::tree_frame(&state);
    assert!(
        frame
            .iter()
            .any(|line| line.starts_with("│  ├─ ● downloading 1/2 (2.0s)")),
        "{frame:?}"
    );
}

#[test]
fn phase_lifecycle_uses_completed_labels_without_counts_or_duplicate_rows() {
    for plain in [false, true] {
        let buffer = Buffer::default();
        let reporter = if plain {
            Reporter::with_plain_writer("Installing Ruby 4.0", buffer.clone())
        } else {
            Reporter::with_writer("Installing Ruby 4.0", buffer.clone())
        };
        let handle = reporter.handle();
        handle.phase_started("Selecting Ruby", "Selected Ruby");
        {
            let state = reporter.progress.lock().unwrap();
            let frame = render::tree_frame(&state).join("\n");
            assert!(frame.contains("│  ├─ ● Selecting Ruby ("), "{frame}");
            assert!(!frame.contains("Selected Ruby"), "{frame}");
        }
        handle.phase_finished("Selecting Ruby", "Selected Ruby", Some("4.0.9".into()));
        {
            let state = reporter.progress.lock().unwrap();
            let frame = render::tree_frame(&state).join("\n");
            assert!(frame.contains("│  ├─ ✓ Selected Ruby 4.0.9 ("), "{frame}");
            assert!(!frame.contains("1/1"), "{frame}");
        }
        handle.phase_started("Downloading", "Downloaded");
        handle.byte_progress("Downloading", 1024, 2048);
        handle.byte_progress("Downloading", 2048, 2048);
        handle.phase_finished("Downloading", "Downloaded", None);
        if !plain {
            let state = reporter.progress.lock().unwrap();
            let frame = render::tree_frame(&state).join("\n");
            assert!(frame.contains("┌─ ● Installing Ruby 4.0"), "{frame}");
            assert!(!frame.contains("Installed Ruby"), "{frame}");
            assert_eq!(frame.matches("Selected Ruby 4.0.9").count(), 1, "{frame}");
            assert!(
                frame.contains("│  ├─ ✓ Downloaded 2.0 KiB / 2.0 KiB (100%)"),
                "{frame}"
            );
        }
        reporter.finish_with_summary("Installed Ruby 4.0.9 in .rubies/ruby-4.0.9");
        let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Selected Ruby 4.0.9"), "{output}");
        assert!(
            output.contains("Downloaded 2.0 KiB / 2.0 KiB (100%)"),
            "{output}"
        );
        assert!(output.contains("Installed Ruby 4.0.9"), "{output}");
        assert!(!output.contains("1/1"), "{output}");
    }
}

#[test]
fn failed_phase_keeps_its_active_label() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("Installing Ruby", buffer.clone());
    let handle = reporter.handle();
    handle.phase_started("Validating installation", "Validated installation");
    handle.fail("Missing standard library");
    reporter.finish();
    let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert!(
        output.contains("│  ├─ × Validating installation"),
        "{output}"
    );
    assert!(output.contains("Missing standard library"), "{output}");
    assert!(!output.contains("Validated installation"), "{output}");
}
