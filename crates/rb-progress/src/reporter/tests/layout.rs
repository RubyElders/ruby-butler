use super::*;

#[test]
fn phases_stay_inside_the_parent_tree_in_live_and_final_output() {
    let buffer = Buffer::default();
    let reporter = Reporter::with_writer("Polishing the silver", buffer.clone());
    reporter.handle().event("polishing", 0, 3, None);
    let frame = render::tree_frame(&reporter.progress.lock().unwrap());
    assert!(frame[1].starts_with("│  ├─ ● polishing 0/3 "), "{frame:?}");
    reporter.handle().event("polishing", 3, 3, None);
    reporter.finish_with_summary("The silver is ready, sir");
    let output = buffer.text();
    assert!(output.contains("\x1b[2K│  ├─ ✓ polishing 3/3 "), "{output}");
}

#[test]
fn nested_phases_and_workers_each_add_one_level() {
    let parent = Reporter::with_writer("Preparing the room", Buffer::default());
    let child = parent.child("Polishing the silver");
    child.handle().event("polishing", 0, 3, None);
    child.handle().worker_started(0, "silverware");
    child.handle().worker_output(0, "one polished spoon");
    let frame = render::tree_frame(&parent.progress.lock().unwrap());
    for prefix in [
        "│  ├─ ● Polishing the silver",
        "│  │  ├─ ● polishing",
        "│  │  │  └─ ● worker 0",
        "\x1b[2m│  │  │     one polished spoon",
    ] {
        assert!(
            frame.iter().any(|line| line.starts_with(prefix)),
            "{frame:?}"
        );
    }
}

#[test]
fn readme_examples_match_the_rendered_output() {
    let silver = Buffer::default();
    let reporter = Reporter::with_writer("Polishing the silver", silver.clone());
    reporter.handle().event("polishing", 3, 3, None);
    reporter.finish_with_summary("The silver is ready, sir");

    let room = Buffer::default();
    let parent = Reporter::with_writer("Preparing the room", room.clone());
    let child = parent.child("Polishing the silver");
    child.handle().event("polishing", 3, 3, None);
    child.finish_with_summary("Silver polished");
    parent.finish_with_summary("The room is ready");

    let readme = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));
    let blocks: Vec<_> = readme.split("```text\n").skip(1).collect();
    assert_eq!(blocks.len(), 2);
    for (block, buffer) in blocks.iter().zip([silver, room]) {
        let expected: Vec<_> = block
            .split("```")
            .next()
            .unwrap()
            .lines()
            .map(without_duration)
            .collect();
        let output = buffer.text();
        let actual: Vec<_> = output
            .split("\x1b[2K")
            .skip(1)
            .map(|part| without_duration(part.split(['\x1b', '\r', '\n']).next().unwrap()))
            .collect();
        assert!(
            actual.windows(expected.len()).any(|rows| rows == expected),
            "{actual:?} != {expected:?}"
        );
    }
}

fn without_duration(line: &str) -> &str {
    line.rsplit_once(" (").map_or(line, |(text, _)| text)
}

#[test]
fn a_failed_child_does_not_change_a_completed_phase_to_failed() {
    let buffer = Buffer::default();
    let parent = Reporter::with_writer("Preparing the room", buffer.clone());
    parent
        .handle()
        .phase_finished("Checking supplies", "Checked supplies", None);
    let child = parent.child("Polishing the silver");
    child.handle().fail("Missing polish");
    child.finish();
    buffer.0.lock().unwrap().clear();
    parent.finish();
    let output = buffer.text();
    assert!(output.contains("┌─ × Preparing the room"), "{output}");
    let final_output = &output[output.rfind("┌─ × Preparing the room").unwrap()..];
    assert!(
        final_output.contains("\x1b[2K│  ├─ ✓ Checked supplies"),
        "{final_output}"
    );
    assert!(output.contains("└─ × failed"), "{output}");
}
