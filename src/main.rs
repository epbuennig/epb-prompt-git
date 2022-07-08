#![feature(let_chains)]

use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::{self, Command},
};

use repo::{Change, Changes};

mod repo;
mod util;

fn get_prompt(path: &Path) -> Result<repo::Prompt, Box<dyn Error>> {
    // use https://git-scm.com/docs/git-status
    let output = Command::new("git")
        .current_dir(path)
        .args([
            "status",
            "--porcelain=v2",
            "--column",
            "--branch",
            "--show-stash",
        ])
        .output()?;

    let lines = String::from_utf8_lossy(&output.stdout);

    let mut commit = None;
    let (mut local, mut remote) = (None, None);
    let (mut ahead, mut behind, mut conflicts, mut stash, mut _ignored) = (0, 0, 0, 0, 0);
    let (mut working_tree, mut index) = (Changes::new(), Changes::new());

    for line in lines.lines().filter(|s| !s.is_empty()) {
        // # branch.oid <commit> | (initial)        Current commit.
        // # branch.head <branch> | (detached)      Current branch.
        // # branch.upstream <upstream>/<branch>    If upstream is set.
        // # branch.ab +<ahead> -<behind>           If upstream is set and the commit is present.
        if let Some(rest) = line.strip_prefix("# branch.") {
            if let Some(oid) = rest.strip_prefix("oid ") {
                commit = (oid != "(initial)").then_some(oid);
                continue;
            }

            if let Some(name) = rest.strip_prefix("head ") {
                local = (name != "(detached)").then_some(name);
                continue;
            }

            if let Some(upstream) = rest.strip_prefix("upstream ") {
                remote = Some(upstream);
                continue;
            }

            if let Some(rest) = rest.strip_prefix("ab +") {
                let (aheadstr, behindstr) = rest.split_once(" -").unwrap();

                ahead = aheadstr.parse().expect("valid count");
                behind = behindstr.parse().expect("valid count");
                continue;
            }
        }

        // # stash <N>  stashed
        if let Some(rest) = line.strip_prefix("# stash ") {
            stash = rest.trim().parse()?;
            continue;
        }

        // ? <path>     untracked
        if line.starts_with("? ") {
            working_tree[Change::Add] += 1;
            continue;
        }

        // ! <path>     ignored
        if line.starts_with("! ") {
            _ignored += 1;
            continue;
        }

        // .x   not updated
        // Mx   updated in index
        // Tx   type changed in index
        // Ax   added to index
        // Dx   deleted from index
        // x.   index and work tree matches
        // xM   work tree changed since index
        // xT   type changed in work tree since index
        // xD   deleted in work tree

        // changes
        if let Some((x, y)) = util::parse_xy_line(line, "1 ") {
            match x {
                '.' => {}
                'A' => index[Change::Add] += 1,
                'M' => index[Change::Mod] += 1,
                'D' => index[Change::Del] += 1,
                'T' => index[Change::Typ] += 1,
                x => eprintln!("idx: {x}"),
            }

            match y {
                '.' => {}
                'A' => working_tree[Change::Add] += 1,
                'M' => working_tree[Change::Mod] += 1,
                'D' => working_tree[Change::Del] += 1,
                'T' => working_tree[Change::Typ] += 1,
                x => eprintln!("idx: {x}"),
            }

            continue;
        }

        // Cx   copied in index
        // Rx   renamed in index
        // xR   renamed in work tree
        // xC   copied in work tree
        if let Some((x, y)) = util::parse_xy_line(line, "2 ") {
            match x {
                '.' => {}
                'R' => index[Change::Ren] += 1,
                'C' => {}
                x => eprintln!("idx: {x}"),
            }

            match y {
                '.' => {}
                'R' => working_tree[Change::Ren] += 1,
                'C' => {}
                x => eprintln!("idx: {x}"),
            }

            continue;
        }

        // DD   both deleted
        // AU   added by us
        // UD   deleted by them
        // UA   added by them
        // DU   deleted by us
        // AA   both added
        // UU   both modified
        if let Some(_) = util::parse_xy_line(line, "u ") {
            conflicts += 1;
            continue;
        }
    }

    // eprintln!("commit:      {:?}", commit);
    // eprintln!("local:       {:?}", local);
    // eprintln!("remote:      {:?}", remote);
    // eprintln!("ab:          {:?}", (ahead, behind));
    // eprintln!("conflict:    {:?}", conflicts);
    // eprintln!("stash:       {:?}", stash);
    // eprintln!("ignore:      {:?}", ignored);
    // eprintln!("wt:          {:?}", working_tree);
    // eprintln!("idx:         {:?}", index);

    let commit = if let Some(commit) = commit {
        commit
    } else {
        return Ok(repo::Prompt::headless(working_tree, index, stash));
    };

    let local = if let Some(local) = local {
        local
    } else {
        // if conflicts are non zero then this may be a detached rebase head
        if conflicts == 0 {
            return Ok(repo::Prompt::detached(
                repo::Commit::new(commit.to_owned()),
                working_tree,
                index,
                stash,
            ));
        } else {
            commit
        }
    };

    let remote_diverge = remote.map(|name| {
        let (remote, branch) = name.split_once('/').unwrap();
        (
            repo::RemoteBranch::new(remote.to_owned(), branch.to_owned()),
            (ahead + behind != 0).then(|| repo::Divergence::new(ahead, behind)),
        )
    });

    if conflicts != 0 {
        let output = Command::new("git")
            .current_dir(path)
            .arg("show-ref")
            .output()?;

        let lines = String::from_utf8_lossy(&output.stdout);

        let ref_buffer; // not read so must not be always init
        let (kind, mut source, mut target) = if let Some(merge_head) =
            util::try_get_file_content(path.join(".git/MERGE_HEAD"))?
        {
            ref_buffer = merge_head;
            (repo::ConflictKind::Merge, local, ref_buffer.as_str())
        } else if let Some(rebase_head) = util::try_get_file_content(path.join(".git/REBASE_HEAD"))?
        {
            ref_buffer = rebase_head;
            (repo::ConflictKind::Rebase, commit, ref_buffer.as_str())
        } else {
            todo!()
        };

        // only use if `refs/heads`?
        // this may need to be recursive
        let (mut is_source_branch, mut is_target_branch) = (false, false);
        for (id, reference) in lines
            .lines()
            .map(|line| line.split_once(' ').expect("<id> <ref>"))
        {
            if id == source {
                source = reference;
                is_source_branch = true;
            } else if id == target {
                target = reference;
                is_target_branch = true;
            }
        }

        fn resolve(reference: &str, is_branch: bool) -> repo::ConflictRef {
            if is_branch {
                repo::ConflictRef::branch(reference.trim_start_matches("refs/heads/").to_owned())
            } else {
                repo::ConflictRef::commit(reference.to_owned())
            }
        }

        return Ok(repo::Prompt::conflict(
            kind,
            resolve(&source, is_source_branch),
            resolve(&target, is_target_branch),
            working_tree,
            index,
            conflicts,
            stash,
        ));
    }

    if working_tree.any() || index.any() {
        return Ok(repo::Prompt::working(
            repo::Branch::new(local.to_owned(), remote_diverge),
            working_tree,
            index,
            stash,
        ));
    }

    return Ok(repo::Prompt::clean(
        repo::Branch::new(local.to_owned(), remote_diverge),
        stash,
    ));
}

fn main() {
    let pwd = env::current_dir().expect("could not acquire pwd");
    let arg_path = env::args_os().nth(1).map(Into::<PathBuf>::into);

    // this will return `pwd` if `arg_path` was `None`
    let path = util::path_rel_to_abs(&pwd, arg_path.as_deref());
    match get_prompt(&*path) {
        Ok(result) => println!("{:#}", result),
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
