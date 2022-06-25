use crate::repo::{Change, Changes, Divergence, RemoteBranch};
use git2::{BranchType, Error, Oid, Reference, Repository, Status};
use std::{borrow::Cow, num::NonZeroUsize, path::Path};

pub const ERROR_NON_UNICODE: &str = "fatal error on resolving reference, was not utf8";
pub const ERROR_NON_DIRECT: &str = "fatal error on resolving reference, was not direct";

pub fn path_rel_to_abs<'p>(pwd: &'p Path, arg_path: Option<&'p Path>) -> Cow<'p, Path> {
    match arg_path {
        Some(path) => {
            if path.is_absolute() {
                Cow::Borrowed(path)
            } else {
                Cow::Owned(pwd.join(path))
            }
        }
        None => Cow::Borrowed(pwd),
    }
}

pub fn try_get_repo_for(path: &Path) -> Option<Repository> {
    debug_assert!(path.is_absolute(), "path should be absolute");

    for ancestor in path.ancestors() {
        match Repository::open(ancestor) {
            Ok(repo) => return Some(repo),
            Err(_) => continue,
        }
    }

    None
}

pub fn get_local_changes(
    repo: &Repository,
) -> Result<(Changes, Changes, Option<NonZeroUsize>), Error> {
    let (mut working_tree, mut index, mut conflicts) = (Changes::new(), Changes::new(), 0usize);

    for status in repo.statuses(None)?.iter() {
        if !matches!(status.status(), Status::CURRENT | Status::IGNORED) {
            if status.status() == Status::CONFLICTED {
                conflicts += 1;
            }

            let (wt_status, idx_status) = Change::new(status.status());

            if let Some(status) = wt_status {
                working_tree[status] += 1;
            }

            if let Some(status) = idx_status {
                index[status] += 1;
            }
        }
    }

    Ok((working_tree, index, NonZeroUsize::new(conflicts)))
}

pub fn try_resolve_oid_to_branch(repo: &Repository, oid: Oid) -> Result<Option<String>, Error> {
    for res in repo.branches(None)? {
        let (branch, _) = res?;
        let reference = branch.into_reference();
        if reference.target().expect(ERROR_NON_DIRECT) == oid {
            return Ok(Some(
                reference.shorthand().expect(ERROR_NON_UNICODE).to_owned(),
            ));
        }
    }

    Ok(None)
}

pub fn resovle_remote_divergence<'repo>(
    repo: &'repo Repository,
    source: &Reference<'repo>,
) -> Result<Option<(RemoteBranch, Option<Divergence>)>, Error> {
    match repo.branch_upstream_name(source.name().expect(ERROR_NON_UNICODE)) {
        Ok(upstream) => {
            let remote_and_branch = upstream
                .as_str()
                .expect(ERROR_NON_UNICODE)
                .trim_start_matches("refs/remotes/")
                .to_owned();

            let target_local = source.target().expect(ERROR_NON_UNICODE);
            let target_remote = repo
                .find_branch(&remote_and_branch, BranchType::Remote)?
                .into_reference()
                .target()
                .expect(ERROR_NON_DIRECT);

            let (ahead, behind) = repo.graph_ahead_behind(target_local, target_remote)?;

            // divergent?
            if ahead != 0 || behind != 0 {
                Ok(Some((
                    RemoteBranch::split(remote_and_branch),
                    Some(Divergence::new(ahead, behind)),
                )))
            } else {
                Ok(Some((RemoteBranch::split(remote_and_branch), None)))
            }
        }
        // no upstream
        Err(_) => Ok(None),
    }
}
