use crate::TaskId;
use std::collections::{BTreeSet, VecDeque};

/// Describes a unit of work and the tasks that must finish before it can run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    pub name: String,
    pub deps: Vec<TaskId>,
}

/// Collects tasks and dependencies for validation and scheduling.
#[derive(Clone, Debug, Default)]
pub struct TaskGraph {
    tasks: Vec<Task>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &mut self,
        name: impl Into<String>,
        deps: impl IntoIterator<Item = TaskId>,
    ) -> TaskId {
        let id = TaskId(self.tasks.len());
        self.tasks.push(Task {
            name: name.into(),
            deps: deps.into_iter().collect(),
        });
        id
    }

    pub fn task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(id.0)
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn schedule(&self) -> Result<Schedule, PlanError> {
        let mut indegree = vec![0usize; self.tasks.len()];
        let mut dependents = vec![Vec::new(); self.tasks.len()];
        for (index, task) in self.tasks.iter().enumerate() {
            for &dependency in &task.deps {
                if dependency.0 >= self.tasks.len() {
                    return Err(PlanError::UnknownDependency(TaskId(index), dependency));
                }
                indegree[index] += 1;
                dependents[dependency.0].push(TaskId(index));
            }
        }
        let ready = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(TaskId(index)))
            .collect();
        let schedule = Schedule {
            graph: self.clone(),
            indegree,
            dependents,
            ready,
            completed: BTreeSet::new(),
            running: BTreeSet::new(),
        };
        if schedule.topological_count() != self.tasks.len() {
            return Err(PlanError::Cycle);
        }
        Ok(schedule)
    }
}

/// Explains why a graph cannot produce a valid schedule.
#[derive(Debug, Eq, PartialEq)]
pub enum PlanError {
    UnknownDependency(TaskId, TaskId),
    Cycle,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownDependency(task, dependency) => write!(
                formatter,
                "task {} has unknown dependency {}",
                task.index(),
                dependency.index()
            ),
            Self::Cycle => formatter.write_str("task graph contains a cycle"),
        }
    }
}

impl std::error::Error for PlanError {}

/// Explains why a task cannot be marked complete in a schedule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionError {
    UnknownTask(TaskId),
    NotRunning(TaskId),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTask(id) => write!(formatter, "unknown task {}", id.index()),
            Self::NotRunning(id) => write!(formatter, "task {} is not running", id.index()),
        }
    }
}

impl std::error::Error for CompletionError {}

/// Tracks ready, running, and completed tasks in a validated dependency graph.
#[derive(Debug)]
pub struct Schedule {
    graph: TaskGraph,
    indegree: Vec<usize>,
    dependents: Vec<Vec<TaskId>>,
    ready: VecDeque<TaskId>,
    completed: BTreeSet<TaskId>,
    running: BTreeSet<TaskId>,
}

impl Schedule {
    pub fn take_ready(&mut self) -> Option<TaskId> {
        let id = self.ready.pop_front()?;
        self.running.insert(id);
        Some(id)
    }

    pub fn task(&self, id: TaskId) -> &Task {
        &self.graph.tasks[id.0]
    }

    pub fn complete(&mut self, id: TaskId) -> Result<bool, CompletionError> {
        if self.graph.task(id).is_none() {
            return Err(CompletionError::UnknownTask(id));
        }
        if self.completed.contains(&id) {
            return Ok(false);
        }
        if !self.running.remove(&id) {
            return Err(CompletionError::NotRunning(id));
        }
        self.completed.insert(id);
        for &dependent in &self.dependents[id.0] {
            self.indegree[dependent.0] -= 1;
            if self.indegree[dependent.0] == 0 {
                self.ready.push_back(dependent);
            }
        }
        Ok(true)
    }

    pub fn is_complete(&self) -> bool {
        self.completed.len() == self.graph.len()
    }

    fn topological_count(&self) -> usize {
        let mut indegree = self.indegree.clone();
        let mut queue: VecDeque<_> = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(TaskId(index)))
            .collect();
        let mut count = 0;
        while let Some(id) = queue.pop_front() {
            count += 1;
            for &dependent in &self.dependents[id.0] {
                indegree[dependent.0] -= 1;
                if indegree[dependent.0] == 0 {
                    queue.push_back(dependent);
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests;
