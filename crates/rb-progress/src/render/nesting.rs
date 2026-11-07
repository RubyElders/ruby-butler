use super::{failure_lines, layout::TreeLayout, own_body_lines};
use crate::{state::ProgressState, terminal::format_duration};

pub(super) fn child_lines(state: &ProgressState, layout: &TreeLayout) -> Vec<String> {
    let mut lines = Vec::new();
    append_children(state, layout, &mut lines);
    lines
}

fn append_children(state: &ProgressState, layout: &TreeLayout, lines: &mut Vec<String>) {
    for child in &state.children {
        let failed = child.failure_count() > 0;
        let marker = if failed {
            "×"
        } else if child.closed {
            "✓"
        } else {
            "●"
        };
        let title = if child.closed && !failed {
            child.summary.as_deref().unwrap_or(&child.title)
        } else {
            &child.title
        };
        lines.push(layout.row(
            "├─",
            &format!("{marker} {title} ({})", format_duration(child.elapsed())),
        ));
        if !child.closed || failed {
            let layout = layout.child();
            append_children(child, &layout, lines);
            lines.extend(own_body_lines(child, &layout));
            lines.extend(failure_lines(child, &layout));
        }
    }
}
