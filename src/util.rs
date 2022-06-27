use crate::repo::{Branch, Change, Changes, ConflictKind, Divergence, RemoteBranch};
use git2::{BranchType, Error, ErrorCode, Oid, Reference, Repository, Status};
use std::{borrow::Cow, path::Path};

pub const ERROR_NON_UNICODE: &str = "fatal error on resolving reference, was not utf8";
pub const ERROR_NON_DIRECT: &str = "fatal error on resolving reference, was not direct";

pub fn path_rel_to_abs<'p>(pwd: &'p Path, arg_path: Option<&'p Path>) -> Cow<'p, Path> {
    debug_assert!(pwd.is_absolute(), "pwd should be absolute");

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

pub fn get_local_changes(repo: &Repository) -> Result<(Changes, Changes, usize), Error> {
    let (mut working_tree, mut index, mut conflicts) = (Changes::new(), Changes::new(), 0usize);

    for status in repo.statuses(None)?.iter() {
        if !matches!(status.status(), Status::CURRENT | Status::IGNORED) {
            if status.status().contains(Status::CONFLICTED) {
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

    Ok((working_tree, index, conflicts))
}

pub fn try_resolve_oid_to_branch(
    repo: &Repository,
    oid: Oid,
) -> Result<Option<git2::Branch<'_>>, Error> {
    for res in repo.branches(None)? {
        let (branch, _) = res?;
        let reference = branch.get();
        if let Some(ref_target) = reference.target() && ref_target == oid {
            return Ok(Some(branch));
        }
    }

    Ok(None)
}

pub fn resolve_remote_divergence<'repo>(
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

            let mut iter = remote_and_branch
                .trim_start_matches("refs/remotes/")
                .split('/');

            let remote = iter
                .next()
                .expect("fatal error on remote branch parse, did not contain remote name")
                .to_owned();

            let branch = iter
                .next()
                .expect("fatal error on remote branch parse, did not contain branch name")
                .to_owned();

            let branch = RemoteBranch::new(remote, branch);

            // divergent?
            if ahead != 0 || behind != 0 {
                Ok(Some((branch, Some(Divergence::new(ahead, behind)))))
            } else {
                Ok(Some((branch, None)))
            }
        }
        // no upstream
        Err(err) if err.code() == ErrorCode::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn try_resolve_conflict_kind(
    repo: &Repository,
) -> Result<(Branch, Branch, ConflictKind), Error> {
    let source = repo.head()?;
    let (kind, target) = match repo.find_reference("MERGE_HEAD") {
        Ok(target) => (ConflictKind::Merge, target),
        Err(err) if err.code() == ErrorCode::NotFound => {
            let rebase = match repo.find_reference("ORIG_HEAD") {
                Ok(rebase) => rebase,
                Err(err) if err.code() == ErrorCode::NotFound => {
                    panic!("fatal error when resolving conflicts, no MERGE_HEAD or ORIG_HEAD found")
                }
                Err(err) => return Err(err),
            };
            (ConflictKind::Rebase, rebase)
        }
        Err(err) => return Err(err.into()),
    };

    let source_local = try_resolve_oid_to_branch(
        &repo,
        source
            .target()
            .expect("fatal error when resolving oid of HEAD"),
    )?
    .expect("fatal error when resolving oid to branch, branch not found");
    let target_local = try_resolve_oid_to_branch(
        &repo,
        target
            .target()
            .expect("fatal error when resolving oid of MERGE_HEAD or ORIG_HEAD"),
    )?
    .expect("fatal error when resolving oid to branch, branch not found");

    let source_local_name = source_local.name()?.expect(ERROR_NON_UNICODE).to_owned();
    let target_local_name = target_local.name()?.expect(ERROR_NON_UNICODE).to_owned();

    let source_remote = resolve_remote_divergence(&repo, &source_local.into_reference())?;
    let target_remote = resolve_remote_divergence(&repo, &target_local.into_reference())?;

    Ok((
        Branch::new(source_local_name, source_remote),
        Branch::new(target_local_name, target_remote),
        kind,
    ))
}
