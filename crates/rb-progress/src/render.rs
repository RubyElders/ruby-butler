mod layout;
mod nesting;

use crate::state::{ProgressState, WorkerStatus};
use crate::terminal::{
    activity_icon, clear_sequence, fit_terminal, format_duration, limit_tree_viewport,
    viewport_height,
};
use layout::TreeLayout;
use nesting::child_lines;

fn print(state: &ProgressState, arguments: std::fmt::Arguments<'_>) {
    if let Some(output) = &state.output {
        let _ = output.lock().unwrap().write_fmt(arguments);
    }
}

fn println(state: &ProgressState, arguments: std::fmt::Arguments<'_>) {
    print(state, arguments);
    print(state, format_args!("\n"));
}

fn flush(state: &ProgressState) {
    if let Some(output) = &state.output {
        let _ = output.lock().unwrap().flush();
    }
}

pub(crate) fn draw(state: &mut ProgressState) {
    if state.plain {
        return;
    }
    if state.phase.is_empty()
        && state.history.is_empty()
        && state.workers.is_empty()
        && state.children.is_empty()
    {
        return;
    }
    if state.external_output {
        if !state.handoff_drawn {
            clear_render(state);
            let lines = tree_frame(state);
            repaint(state, &lines);
            let active_prefix = TreeLayout::root().row("├─", "●");
            let active_row = lines
                .iter()
                .position(|line| line.starts_with(&active_prefix))
                .unwrap_or(1);
            let active_offset = lines.len() - active_row - 1;
            if active_offset > 0 {
                print(state, format_args!("\x1b[{}A\r", active_offset));
            }
            state.handoff_drawn = true;
            state.handoff_phase = Some(state.phase);
            state.handoff_tail_rows = lines.len() - active_row;
            state.rendered_lines = 0;
            state.frame_capacity = 0;
            flush(state);
        }
        return;
    }
    let lines = tree_frame(state);
    repaint(state, &lines);
    flush(state);
}

pub(crate) fn tree_frame(state: &ProgressState) -> Vec<String> {
    let elapsed = state.elapsed();
    let icon = activity_icon(elapsed);
    let header = fit_terminal(&format!("┌─ ● {}", state.title));
    let mut body = body_lines(state);
    if body.is_empty() {
        body.push("│  · working…".into());
    }
    let footer =
        if (state.phase.is_empty() && state.children.is_empty()) || !state.workers.is_empty() {
            fit_terminal(&format!(
                "└─ {icon} completed {} | active {} | failed {} ({})",
                state.completed_count(),
                state.active_count(),
                state.failure_count(),
                format_duration(elapsed)
            ))
        } else {
            fit_terminal(&format!("└─ {icon} elapsed {}", format_duration(elapsed)))
        };
    limit_tree_viewport(header, body, footer, viewport_height())
}

pub(crate) fn body_lines(state: &ProgressState) -> Vec<String> {
    let layout = TreeLayout::root();
    let mut lines = child_lines(state, &layout);
    lines.extend(own_body_lines(state, &layout));
    lines
}

fn own_body_lines(state: &ProgressState, layout: &TreeLayout) -> Vec<String> {
    let mut lines = history_lines(state, layout);
    if !state.phase.is_empty() {
        lines.push(active_phase_line(
            state,
            layout,
            if state.closed && state.failure_count() > 0 {
                "×"
            } else {
                "●"
            },
        ));
    }
    let worker_layout = if state.phase.is_empty() {
        None
    } else {
        Some(layout.child())
    };
    let worker_layout = worker_layout.as_ref().unwrap_or(layout);
    for (position, (worker, worker_state)) in state.workers.iter().enumerate() {
        let branch = if position + 1 == state.workers.len() {
            "└─"
        } else {
            "├─"
        };
        let detail = worker_state
            .detail
            .as_deref()
            .map_or(String::new(), |detail| format!(" | {detail}"));
        let marker = match worker_state.status {
            WorkerStatus::Running => "●",
            WorkerStatus::Succeeded => "✓",
            WorkerStatus::Failed => "×",
        };
        lines.push(worker_layout.row(
            branch,
            &format!(
                "{marker} worker {} ({}){} ({})",
                worker,
                worker_state.phase,
                detail,
                format_duration(worker_state.elapsed.map_or_else(
                    || worker_state.started.elapsed(),
                    |elapsed| if worker_state.status != WorkerStatus::Running {
                        elapsed
                    } else {
                        elapsed + worker_state.started.elapsed()
                    }
                ),)
            ),
        ));
        for line in &worker_state.output {
            lines.push(worker_layout.preview(line));
        }
    }
    lines
}

fn failure_lines(state: &ProgressState, layout: &TreeLayout) -> Vec<String> {
    let error_layout = if state.phase.is_empty() {
        None
    } else {
        Some(layout.child())
    };
    let error_layout = error_layout.as_ref().unwrap_or(layout);
    let branch = if state.phase.is_empty() {
        "├─"
    } else {
        "└─"
    };
    state
        .failures
        .iter()
        .flat_map(|error| error_layout.error(branch, error))
        .collect()
}

fn history_lines(state: &ProgressState, layout: &TreeLayout) -> Vec<String> {
    state
        .history
        .iter()
        .filter(|record| record.phase != state.phase)
        .map(|record| {
            phase_line(
                layout,
                "✓",
                record.completed_label.unwrap_or(record.phase),
                Some(record.unit.format(record.done, record.total)),
                record.detail.as_deref(),
                record.elapsed,
            )
        })
        .collect()
}

fn active_phase_line(state: &ProgressState, layout: &TreeLayout, marker: &str) -> String {
    let stats = (state.total > 0
        || matches!(state.unit, crate::progress_unit::ProgressUnit::Bytes))
    .then(|| state.unit.format(state.done, state.total));
    phase_line(
        layout,
        if state.phase_finished { "✓" } else { marker },
        if state.phase_finished {
            state.completed_label.unwrap_or(state.phase)
        } else {
            state.phase
        },
        stats,
        state.phase_detail.as_deref(),
        state.phase_elapsed(),
    )
}

fn phase_line(
    layout: &TreeLayout,
    marker: &str,
    phase: &str,
    stats: Option<String>,
    detail: Option<&str>,
    elapsed: std::time::Duration,
) -> String {
    let uncounted = stats.as_ref().is_none_or(String::is_empty);
    let stats = stats
        .filter(|stats| !stats.is_empty())
        .map_or(String::new(), |stats| format!(" {stats}"));
    let detail = detail.map_or(String::new(), |detail| {
        if uncounted {
            format!(" {detail}")
        } else {
            format!(" | {detail}")
        }
    });
    layout.row(
        "├─",
        &format!(
            "{marker} {phase}{stats}{detail} ({})",
            format_duration(elapsed)
        ),
    )
}

fn repaint(state: &mut ProgressState, lines: &[String]) {
    let first_frame = state.rendered_lines == 0 && !state.anchor_started;
    if state.rendered_lines > 0 {
        print(
            state,
            format_args!("\x1b[{}A\r", state.rendered_lines.saturating_sub(1)),
        );
    } else if first_frame {
        print(state, format_args!("\r"));
        state.anchor_started = true;
    } else {
        print(state, format_args!("\r"));
    }
    if state.frame_capacity > 0 {
        print(state, format_args!("\x1b[{}M", state.frame_capacity));
    }
    if first_frame {
        if let Some(first) = lines.first() {
            print(state, format_args!("\x1b[2K{}", first));
        }
        if lines.len() > 1 {
            print(state, format_args!("\n"));
            print(state, format_args!("\x1b[{}L", lines.len() - 1));
            for (index, line) in lines.iter().skip(1).enumerate() {
                print(state, format_args!("\x1b[2K{}", line));
                if index + 1 < lines.len() - 1 {
                    print(state, format_args!("\n"));
                }
            }
        }
    } else {
        print(state, format_args!("\x1b[{}L", lines.len()));
        for (index, line) in lines.iter().enumerate() {
            print(state, format_args!("\x1b[2K{}", line));
            if index + 1 < lines.len() {
                print(state, format_args!("\n"));
            }
        }
    }
    state.rendered_lines = lines.len();
    state.frame_capacity = lines.len();
}

fn clear_render(state: &mut ProgressState) {
    if state.rendered_lines == 0 {
        return;
    }
    print(
        state,
        format_args!("{}", clear_sequence(state.rendered_lines)),
    );
    if state.frame_capacity > 0 {
        print(state, format_args!("\x1b[{}M", state.frame_capacity));
    }
    state.rendered_lines = 0;
    state.frame_capacity = 0;
}

pub(crate) fn finish_compact(state: &mut ProgressState, summary: Option<String>) {
    if state.plain {
        crate::plain::finish(state, summary.as_deref());
        return;
    }
    clear_render(state);
    let line = fit_terminal(&format!(
        "✓ {} ({})",
        summary.as_deref().unwrap_or("complete"),
        format_duration(state.elapsed())
    ));
    println(state, format_args!("{line}"));
    flush(state);
}

pub(crate) fn finish(state: &mut ProgressState, summary: Option<String>) {
    if state.plain {
        crate::plain::finish(state, summary.as_deref());
        return;
    }
    if state.started.is_none() {
        return;
    }
    if state.handoff_drawn {
        let layout = TreeLayout::root();
        let line = layout.row(
            "├─",
            &format!(
                "✓ {} | {} done ({})",
                state.title,
                state.phase,
                format_duration(state.phase_elapsed())
            ),
        );
        print(state, format_args!("\r"));
        for index in 0..state.handoff_tail_rows {
            print(state, format_args!("\x1b[2K"));
            if index + 1 < state.handoff_tail_rows {
                println(state, format_args!(""));
            }
        }
        if state.handoff_tail_rows > 1 {
            print(
                state,
                format_args!("\x1b[{}A\r", state.handoff_tail_rows - 1),
            );
        }
        print(state, format_args!("\x1b[{}M", state.handoff_tail_rows));
        if let Some(phase) = state.handoff_phase
            && let Some(record) = state
                .history
                .iter()
                .rev()
                .find(|record| record.phase == phase)
        {
            let stats = if record.total > 0 {
                record.unit.format(record.done, record.total)
            } else {
                "done".into()
            };
            let row = phase_line(
                &layout,
                "✓",
                record.completed_label.unwrap_or(phase),
                Some(stats),
                record.detail.as_deref(),
                record.elapsed,
            );
            println(state, format_args!("{row}"));
        }
        println(state, format_args!("{}", line));
        if let Some(summary) = summary {
            println(state, format_args!("└─ ✓ {}", fit_terminal(&summary)));
        }
        state.handoff_drawn = false;
        state.handoff_phase = None;
        state.handoff_tail_rows = 0;
        state.rendered_lines = 0;
        state.frame_capacity = 0;
    } else {
        let failed = state.failure_count() > 0;
        let marker = if failed { "×" } else { "✓" };
        let header = fit_terminal(&format!("┌─ {marker} {}", state.title));
        let layout = TreeLayout::root();
        let mut body = child_lines(state, &layout);
        body.extend(history_lines(state, &layout));
        if !state.phase.is_empty() {
            body.push(active_phase_line(state, &layout, marker));
        }
        body.extend(failure_lines(state, &layout));
        let compact_error = failed && viewport_height() <= 3;
        let footer = fit_terminal(&format!(
            "└─ {marker} {} ({})",
            if compact_error {
                state.last_failure().unwrap_or("failed")
            } else {
                summary
                    .as_deref()
                    .unwrap_or(if failed { "failed" } else { "complete" })
            },
            format_duration(state.elapsed())
        ));
        let limit = viewport_height().saturating_sub(1).max(2);
        if body.len() > limit.saturating_sub(2) {
            body.drain(..body.len() - limit.saturating_sub(2));
        }
        let lines = limit_tree_viewport(header, body, footer, limit);
        repaint(state, &lines);
        println(state, format_args!(""));
        state.rendered_lines = 0;
        state.frame_capacity = 0;
    }
    flush(state);
}
