use crate::state::{ProgressState, WorkerStatus};
use crate::terminal::{
    activity_icon, clear_sequence, fit_terminal, format_duration, limit_tree_viewport,
    viewport_height,
};

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
    if state.phase.is_empty() && state.history.is_empty() && state.workers.is_empty() {
        return;
    }
    if state.external_output {
        if !state.handoff_drawn {
            clear_render(state);
            let lines = tree_frame(state);
            repaint(state, &lines);
            let active_row = lines
                .iter()
                .position(|line| line.starts_with("│  ├─ ●"))
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
    let footer = if state.phase.is_empty() {
        let active = state
            .workers
            .values()
            .filter(|worker| worker.status == WorkerStatus::Running)
            .count();
        fit_terminal(&format!(
            "└─ {icon} completed {} | active {} | failed {} ({})",
            state.completed_tasks,
            active,
            state.failed_tasks,
            format_duration(elapsed)
        ))
    } else {
        fit_terminal(&format!(
            "└─ {icon} {} {}/{} | workers {} ({})",
            state.phase,
            state.done,
            state.total,
            state.workers.len(),
            format_duration(elapsed)
        ))
    };
    limit_tree_viewport(header, body, footer, viewport_height())
}

pub(crate) fn body_lines(state: &ProgressState) -> Vec<String> {
    let mut lines = state
        .history
        .iter()
        .map(|record| {
            let stats = if record.total > 0 {
                format!("{}/{}", record.done, record.total)
            } else {
                "done".into()
            };
            format!(
                "│  ├─ ✓ {} {} ({})",
                record.phase,
                stats,
                format_duration(record.elapsed)
            )
        })
        .collect::<Vec<_>>();
    if !state.phase.is_empty() {
        lines.push(format!(
            "│  ├─ ● {} ({})",
            state.phase,
            format_duration(state.phase_elapsed())
        ));
    }
    let mut lines: Vec<_> = lines.into_iter().map(|line| fit_terminal(&line)).collect();
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
        lines.push(fit_terminal(&format!(
            "│     {} {marker} worker {} ({}){} ({})",
            branch,
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
        )));
        for line in &worker_state.output {
            let line = fit_terminal(&format!("│        {line}"));
            lines.push(format!("\x1b[2m{line}\x1b[22m"));
        }
    }
    lines
}

#[cfg(test)]
pub(crate) fn history_lines(state: &ProgressState) -> Vec<String> {
    state
        .history
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let stats = if record.total > 0 {
                format!("{}/{}", record.done, record.total)
            } else {
                "done".into()
            };
            let detail = record
                .detail
                .as_deref()
                .map_or(String::new(), |detail| format!(" | {detail}"));
            fit_terminal(&format!(
                "{} ✓ {} {}{} ({})",
                if index == 0 { "┌─" } else { "├─" },
                record.phase,
                stats,
                detail,
                format_duration(record.elapsed)
            ))
        })
        .collect()
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
        let line = fit_terminal(&format!(
            "├─ ✓ {} | {} done ({})",
            state.title,
            state.phase,
            format_duration(state.phase_elapsed())
        ));
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
                format!("{}/{}", record.done, record.total)
            } else {
                "done".into()
            };
            println(
                state,
                format_args!(
                    "├─ ✓ {} {} ({})",
                    phase,
                    stats,
                    format_duration(record.elapsed)
                ),
            );
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
        let failed = state.failed_tasks > 0;
        let marker = if failed { "×" } else { "✓" };
        let header = fit_terminal(&format!("┌─ {marker} {}", state.title));
        let mut body = state
            .history
            .iter()
            .map(|record| {
                fit_terminal(&format!(
                    "│  ├─ ✓ {} {}/{}{} ({})",
                    record.phase,
                    record.done,
                    record.total,
                    record
                        .detail
                        .as_deref()
                        .map_or(String::new(), |detail| format!(" | {detail}")),
                    format_duration(record.elapsed)
                ))
            })
            .collect::<Vec<_>>();
        if !state.phase.is_empty() {
            body.push(fit_terminal(&format!(
                "│  ├─ {marker} {} {}/{}{} ({})",
                state.phase,
                state.done,
                state.total,
                state
                    .phase_detail
                    .as_deref()
                    .map_or(String::new(), |detail| format!(" | {detail}")),
                format_duration(state.phase_elapsed())
            )));
        }
        let failure_lines = state
            .failures
            .iter()
            .flat_map(|error| {
                error
                    .lines()
                    .map(|line| fit_terminal(&format!("│  × {line}")))
            })
            .collect::<Vec<_>>();
        body.extend(failure_lines);
        let compact_error = failed && viewport_height() <= 3;
        let footer = fit_terminal(&format!(
            "└─ {marker} {} ({})",
            if compact_error {
                state
                    .failures
                    .last()
                    .map(String::as_str)
                    .unwrap_or("failed")
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
