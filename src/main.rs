#![feature(let_chains)]
#![feature(try_find)]

use repo::{Branch, Commit, Repo};
use std::{env, error::Error, path::PathBuf, process};

mod repo;
pub mod util;

#[derive(Debug)]
enum ExecError {
    NoRepo,
    Other(Box<dyn Error>),
}

impl<E: Error + 'static> From<E> for ExecError {
    fn from(error: E) -> Self {
        Self::Other(Box::new(error))
    }
}

fn exec() -> Result<Repo, ExecError> {
    let pwd = env::current_dir().expect("could not acquire pwd");
    let arg_path = env::args_os().nth(1).map(Into::<PathBuf>::into);

    // this will return `pwd` if `arg_path` was `None`
    let path = util::path_rel_to_abs(&pwd, arg_path.as_deref());
    let repo = util::try_get_repo_for(&path).ok_or(ExecError::NoRepo)?;
    let (working_tree, index, conflicts) = util::get_local_changes(&repo)?;

    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => return Ok(Repo::headless(working_tree, index)),
    };

    if let Some(conflicts) = conflicts {
        let source = repo.find_reference("ORIG_HEAD")?;
        let target = repo.find_reference("MERGE_HEAD")?;

        let source_local = util::try_resolve_oid_to_branch(
            &repo,
            source
                .target()
                .expect("fata error when resolving oid of ORIG_HEAD"),
        )?
        .expect("fatal error when resolving oid to branch, branch not found");
        let target_local = util::try_resolve_oid_to_branch(
            &repo,
            target
                .target()
                .expect("fata error when resolving oid of MERGE_HEAD"),
        )?
        .expect("fatal error when resolving oid to branch, branch not found");

        let source_remote = util::resovle_remote_divergence(&repo, &source)?;
        let target_remote = util::resovle_remote_divergence(&repo, &target)?;

        return Ok(Repo::conflict(
            Branch::new(source_local, source_remote),
            Branch::new(target_local, target_remote),
            working_tree,
            index,
            conflicts,
        ));
    }

    if repo.head_detached()? {
        return Ok(Repo::detached(
            Commit::new(head.target().expect(util::ERROR_NON_DIRECT).to_string()),
            working_tree,
            index,
        ));
    }

    // get remote if not detached, if exists, get ahead behind
    let remote = util::resovle_remote_divergence(&repo, &head)?;

    // we're not detached so shorthand should give us the local branch name
    let branch = Branch::new(
        head.shorthand().expect(util::ERROR_NON_UNICODE).to_owned(),
        remote,
    );

    Ok(if !working_tree.any() && !index.any() {
        Repo::clean(branch)
    } else {
        Repo::working(branch, working_tree, index)
    })
}

fn main() {
    match exec() {
        Ok(result) => println!("{:#}", result),
        Err(err) => {
            println!(
                "[{fg}error{n}]",
                fg = termion::color::Fg(termion::color::Red),
                n = termion::style::Reset
            );

            if let Some("--debug") = env::args().nth(2).as_deref() {
                eprintln!("{err:?}");
            }

            process::exit(1)
        }
    };
}
