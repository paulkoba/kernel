use crate::fs::dentry::Dentry;

type DentryInitFunc = fn(dentry: &mut Dentry) -> i32;
type DentryReleaseFunc = fn(dentry: &mut Dentry);

pub struct DentryOperations {
    pub dentry_init: Option<DentryInitFunc>,
    pub dentry_release: Option<DentryReleaseFunc>,
}
