use crate::{
    events::ProgressEvent,
    state::ProgressState,
    terminal::{format_duration, plain_text},
};

pub(crate) fn line(state: &ProgressState, prefix: &str, text: &str) {
    if let Some(output) = &state.output {
        let mut output = output.lock().unwrap();
        for text in text.split('\n') {
            let _ = writeln!(output, "[{prefix}] {}", plain_text(text));
        }
        let _ = output.flush();
    }
}

pub(crate) fn progress(state: &ProgressState, event: &ProgressEvent) {
    let prefix = event
        .worker
        .map_or_else(|| "overall".into(), |worker| format!("worker: {worker}"));
    let detail = event.detail.as_deref().unwrap_or("");
    line(
        state,
        &prefix,
        &format!("{} {}/{} {detail}", event.phase, event.done, event.total),
    );
}

pub(crate) fn finish(state: &ProgressState, summary: Option<&str>) {
    let status = if state.failed_tasks > 0 {
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
            state.completed_tasks,
            state.failed_tasks,
            format_duration(state.elapsed())
        ),
    );
}
