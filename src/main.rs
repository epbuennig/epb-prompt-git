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
        let (source, target, kind) = util::try_resolve_conflict_kind(&repo)?;
        return Ok(Repo::conflict(
            kind,
            source,
            target,
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
    let remote = util::resolve_remote_divergence(&repo, &head)?;

    // we're not detached so shorthand should give us the local branch name
    let branch = Branch::new(
        head.shorthand().expect(util::ERROR_NON_UNICODE).to_owned(),
        remote,
    );

    Ok(if working_tree.any() || index.any() {
        Repo::working(branch, working_tree, index)
    } else {
        Repo::clean(branch)
    })
}

fn main() {
    match exec() {
        Ok(result) => println!("{:#}", result),
        Err(err) if matches!(err, ExecError::NoRepo) => {
            println!(
                "[{}no repo{}]",
                termion::color::Fg(termion::color::Red),
                termion::style::Reset
            )
        }
        Err(err) => {
            println!(
                "[{}{}error{}]",
                termion::style::Bold,
                termion::color::Fg(termion::color::Red),
                termion::style::Reset
            );

            if let Some("--debug") = env::args().nth(2).as_deref() {
                eprintln!("{err:?}");
            }

            process::exit(1)
        }
    };
}
