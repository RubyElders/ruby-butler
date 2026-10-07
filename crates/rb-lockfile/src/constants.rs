pub(crate) const SECTION_INDENT: usize = 2;
pub(crate) const PACKAGE_INDENT: usize = 4;
pub(crate) const DEPENDENCY_INDENT: usize = 6;
pub(crate) const ENVIRONMENT_INDENT: usize = 3;
pub(crate) const SHA256_HEX_LENGTH: usize = 64;
pub(crate) const GIT_REVISION_HEX_LENGTHS: [usize; 2] = [40, 64];
pub(crate) const SOURCE_OPTION_ORDER: [&str; 6] =
    ["revision", "ref", "branch", "tag", "submodules", "glob"];
