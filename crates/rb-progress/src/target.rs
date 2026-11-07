use crate::{render, state::ProgressState};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, LockResult, Mutex, MutexGuard, PoisonError},
};

#[derive(Clone)]
pub(crate) struct ProgressTarget {
    root: Arc<Mutex<ProgressState>>,
    path: Vec<usize>,
}

impl ProgressTarget {
    pub(crate) fn new(state: ProgressState) -> Self {
        Self {
            root: Arc::new(Mutex::new(state)),
            path: Vec::new(),
        }
    }

    pub(crate) fn lock(&self) -> LockResult<ProgressGuard<'_>> {
        let guard = |root| ProgressGuard {
            root,
            path: &self.path,
        };
        self.root
            .lock()
            .map(guard)
            .map_err(|error| PoisonError::new(guard(error.into_inner())))
    }

    pub(crate) fn child(&self, title: String) -> Self {
        let mut state = self.lock().unwrap();
        let mut scope = state.scope.clone();
        scope.push(title.clone());
        let child = ProgressState {
            title,
            plain: state.plain,
            output: state.output.clone(),
            started: Some(std::time::Instant::now()),
            closed: state.closed,
            scope,
            ..Default::default()
        };
        if child.plain && !child.closed {
            crate::plain::line(&child, "overall", &child.title);
        }
        let mut path = self.path.clone();
        path.push(state.children.len());
        state.children.push(child);
        state.draw();
        Self {
            root: self.root.clone(),
            path,
        }
    }
}

pub(crate) struct ProgressGuard<'a> {
    root: MutexGuard<'a, ProgressState>,
    path: &'a [usize],
}

impl ProgressGuard<'_> {
    pub(crate) fn draw(&mut self) {
        if !self.root.closed {
            render::draw(&mut self.root);
        }
    }

    pub(crate) fn suspend(&mut self) {
        self.root.external_output = true;
        self.draw();
    }

    pub(crate) fn is_root(&self) -> bool {
        self.path.is_empty()
    }
}

impl Deref for ProgressGuard<'_> {
    type Target = ProgressState;

    fn deref(&self) -> &Self::Target {
        let mut state = &*self.root;
        for index in self.path {
            state = &state.children[*index];
        }
        state
    }
}

impl DerefMut for ProgressGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let mut state = &mut *self.root;
        for index in self.path {
            state = &mut state.children[*index];
        }
        state
    }
}
