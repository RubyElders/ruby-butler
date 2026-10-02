use super::*;

#[test]
fn ticker_line_is_bounded() {
    assert!(fit_terminal(&"x".repeat(500)).ends_with('…'));
}

#[test]
fn clearing_returns_to_top_of_rendered_block() {
    let sequence = clear_sequence(4);
    assert!(sequence.ends_with("\x1b[3A\r"));
    assert_eq!(sequence.matches("\x1b[2K").count(), 4);
}

#[test]
fn short_durations_show_progress_before_one_second() {
    assert_eq!(format_duration(Duration::from_millis(500)), "0.5s");
    assert_eq!(format_duration(Duration::from_secs(2)), "2.0s");
}

#[test]
fn viewport_keeps_active_root_and_footer_visible() {
    let visible = limit_tree_viewport(
        "┌─ processing documents".into(),
        vec![
            "│  ✓ resolved".into(),
            "│  ● worker 0".into(),
            "│  ● worker 1".into(),
        ],
        "└─ overall: working".into(),
        4,
    );
    assert_eq!(
        visible,
        [
            "┌─ processing documents",
            "│  ● worker 0",
            "│  ● worker 1",
            "└─ overall: working"
        ]
    );
}

#[test]
fn viewport_drops_old_history_before_active_workers() {
    let visible = limit_tree_viewport(
        "┌─ processing documents".into(),
        vec![
            "│  ✓ resolved".into(),
            "│  ✓ downloaded".into(),
            "│  ● worker 0".into(),
            "│     output".into(),
        ],
        "└─ overall: working".into(),
        4,
    );
    assert_eq!(
        visible,
        [
            "┌─ processing documents",
            "│  ● worker 0",
            "│     output",
            "└─ overall: working"
        ]
    );
}

#[test]
fn viewport_always_leaves_one_line_for_footer() {
    let visible = limit_tree_viewport(
        "┌─ processing documents".into(),
        vec!["│  ● worker 0".into()],
        "└─ overall: working".into(),
        2,
    );
    assert_eq!(visible, ["┌─ processing documents", "└─ overall: working"]);
}

#[test]
fn viewport_preserves_all_rows_when_they_fit() {
    let visible = limit_tree_viewport(
        "┌─ processing documents".into(),
        vec!["│  ✓ resolved".into(), "│  ● worker 0".into()],
        "└─ overall: working".into(),
        4,
    );
    assert_eq!(
        visible,
        [
            "┌─ processing documents",
            "│  ✓ resolved",
            "│  ● worker 0",
            "└─ overall: working"
        ]
    );
}

#[test]
fn tree_viewport_keeps_header_and_footer() {
    let lines = limit_tree_viewport(
        "┌─ ● preparing demo bundle".into(),
        (0..8)
            .map(|index| format!("│  ├─ detail {index}"))
            .collect(),
        "└─ overall: working".into(),
        5,
    );
    assert_eq!(lines.len(), 5);
    assert!(lines[0].starts_with("┌─"));
    assert!(lines[4].starts_with("└─"));
}

#[test]
fn tree_viewport_collapses_body_to_working_marker() {
    let lines = limit_tree_viewport(
        "┌─ ● preparing demo bundle".into(),
        (0..8)
            .map(|index| format!("│  ├─ detail {index}"))
            .collect(),
        "└─ overall: working".into(),
        3,
    );
    assert_eq!(
        lines,
        vec![
            "┌─ ● preparing demo bundle",
            "│  · working…",
            "└─ overall: working",
        ]
    );
}

#[test]
fn truncation_counts_terminal_columns() {
    assert_eq!(fit_width("資料資料資料", 7), "資料資…");
    assert_eq!(
        fit_width("e\u{301}e\u{301}e\u{301}", 3),
        "e\u{301}e\u{301}e\u{301}"
    );
    assert_eq!(fit_width("data", 0), "");
}

#[test]
fn task_text_cannot_move_the_cursor_or_create_extra_rows() {
    assert_eq!(
        fit_width("\x1b[2Jhello\nthere\r\x1b]0;title\x07!", 80),
        "hello there !"
    );
}

#[test]
fn duration_formatting_is_owned_by_the_renderer() {
    assert_eq!(format_duration(Duration::from_millis(2260)), "2.3s");
    assert_eq!(format_duration(Duration::from_millis(12250)), "12s");
    assert_eq!(format_duration(Duration::from_millis(62250)), "1m02s");
}

#[test]
fn activity_icon_cycles_at_the_refresh_interval() {
    let frames = (0..10)
        .map(|tick| activity_icon(Duration::from_millis(tick * 250)))
        .collect::<String>();
    assert_eq!(frames, "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    assert_eq!(activity_icon(Duration::from_millis(249)), '⠋');
    assert_eq!(activity_icon(Duration::from_millis(2500)), '⠋');
    assert!(frames.chars().all(|frame| frame.width() == Some(1)));
}
