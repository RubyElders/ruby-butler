use crate::{
    events::ProgressEvent,
    progress_unit::ProgressUnit,
    state::ProgressState,
    terminal::{format_duration, plain_text},
};

pub(crate) fn line(state: &ProgressState, prefix: &str, text: &str) {
    if let Some(output) = &state.output {
        let mut output = output.lock().unwrap();
        let prefix = if state.scope.is_empty() {
            prefix.to_owned()
        } else {
            format!("{} | {prefix}", state.scope.join(" / "))
        };
        for text in text.split('\n') {
            let _ = writeln!(output, "[{}] {}", plain_text(&prefix), plain_text(text));
        }
        let _ = output.flush();
    }
}

pub(crate) fn progress(state: &ProgressState, event: &ProgressEvent, unit: ProgressUnit) {
    let prefix = event
        .worker
        .map_or_else(|| "overall".into(), |worker| format!("worker: {worker}"));
    let detail = event.detail.as_deref().unwrap_or("");
    let phase = if event.worker.is_none() && state.phase_finished {
        state.completed_label.unwrap_or(event.phase)
    } else {
        event.phase
    };
    let stats = unit.format(event.done, event.total);
    let text = [phase, stats.as_str(), detail]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    line(state, &prefix, &text);
}

pub(crate) fn finish(state: &ProgressState, summary: Option<&str>) {
    let status = if state.failure_count() > 0 {
        "failed"
    } else {
        "complete"
    };
    let detail = summary.map_or(String::new(), |summary| format!(": {summary}"));
    line(
        state,
        "overall",
        &format!(
            "{status}{detail} | completed {} | failed {} ({})",
            state.completed_count(),
            state.failure_count(),
            format_duration(state.elapsed())
        ),
    );
}
