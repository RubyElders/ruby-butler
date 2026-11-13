use crate::{InstallOptions, RubyEnvironment, slot::Slots};
use anyhow::Result;
use rb_task::{Executor, TaskAction, TaskEvents, TaskGraph};
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub(crate) fn restore(
    slots: &Slots,
    home: &Path,
    runtime: &RubyEnvironment,
    options: &InstallOptions,
    events: TaskEvents,
) -> Result<()> {
    let Some(cache) = &options.cache else {
        return Ok(());
    };
    let mut graph = TaskGraph::new();
    let mut actions: BTreeMap<_, TaskAction> = BTreeMap::new();
    for (name, slot) in slots.iter() {
        if slot.lock().unwrap().reused {
            continue;
        }
        let id = graph.add(format!("restore {name}"), []);
        let slot = slot.clone();
        let home = home.to_owned();
        let runtime = runtime.clone();
        let cache = cache.clone();
        let name = name.clone();
        actions.insert(
            id,
            Arc::new(move |context| {
                let mut slot = slot.lock().unwrap();
                let directory = slot.directory.clone();
                let record = slot.record.clone();
                if let Some(entry) = crate::installation_cache::Entry::try_open(
                    &cache,
                    &slot.receipt,
                    directory.file_name().unwrap().to_str().unwrap(),
                    &runtime,
                )
                .map_err(|error| format!("{error:#}"))?
                    && entry
                        .restore(&mut slot, &home, &record, &runtime)
                        .map_err(|error| format!("{error:#}"))?
                {
                    context.progress("reuse", 1, 1, Some(name.clone()));
                }
                Ok(())
            }),
        );
    }
    Executor::new(options.workers, events).run(graph, actions)?;
    Ok(())
}
