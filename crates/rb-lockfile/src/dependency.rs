use rb_gem_types::Requirement;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    pub requirement: Requirement,
    pub pinned: bool,
}
