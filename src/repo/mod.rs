use std::{
    fmt::{Debug, Display, Write},
    num::NonZeroUsize,
    ops::Deref,
};

mod branch;
pub use branch::{Branch, Divergence, RemoteBranch};

mod change;
pub use change::{Change, Changes};

#[derive(Clone, PartialEq, Eq)]
pub struct Commit(String);

impl Commit {
    pub fn new(hash: String) -> Self {
        assert_eq!(hash.len(), 40, "commit hash must be 40 chars long");
        Self(hash)
    }
}

impl Debug for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        // don't use width here because that is expected to add whitespace for values longer than
        // our fmt?
        let len = f
            .width()
            .map(|p| Ord::min(p, self.0.len()))
            .unwrap_or(self.0.len());

        if f.alternate() {
            write!(
                f,
                "{}{}{hash}{}",
                style::Bold,
                color::Fg(color::Yellow),
                style::Reset,
                hash = &self.0[..len]
            )
        } else {
            write!(f, "{hash}", hash = &self.0[..len])
        }
    }
}

impl Deref for Commit {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Merge,
    Rebase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repo {
    Headless {
        working_tree: Changes,
        index: Changes,
    },
    Clean {
        head: Branch,
    },
    Detached {
        head: Commit,
        working_tree: Changes,
        index: Changes,
    },
    Working {
        branch: Branch,
        working_tree: Changes,
        index: Changes,
    },
    Conflicted {
        kind: ConflictKind,
        orig: Branch,
        merge: Branch,
        working_tree: Changes,
        index: Changes,
        conflicts: NonZeroUsize,
    },
}

impl Repo {
    pub fn headless(working_tree: Changes, index: Changes) -> Self {
        Self::Headless {
            working_tree,
            index,
        }
    }

    pub fn clean(branch: Branch) -> Self {
        Self::Clean { head: branch }
    }

    pub fn detached(commit: Commit, working_tree: Changes, index: Changes) -> Self {
        Self::Detached {
            head: commit,
            working_tree,
            index,
        }
    }

    pub fn working(branch: Branch, working_tree: Changes, index: Changes) -> Self {
        Self::Working {
            branch,
            working_tree,
            index,
        }
    }

    pub fn conflict(
        kind: ConflictKind,
        source: Branch,
        target: Branch,
        working_tree: Changes,
        index: Changes,
        conflicts: NonZeroUsize,
    ) -> Self {
        Self::Conflicted {
            kind,
            orig: source,
            merge: target,
            working_tree,
            index,
            conflicts,
        }
    }
}

fn fmt_changes(
    f: &mut std::fmt::Formatter<'_>,
    working_tree: &Changes,
    index: &Changes,
    conflicts: Option<NonZeroUsize>,
) -> std::fmt::Result {
    use termion::{color, style};

    if working_tree.any() || index.any() || conflicts.is_some() {
        f.write_str(" ::")?;
    }

    if let Some(conflicts) = conflicts {
        if f.alternate() {
            write!(
                f,
                " [{}{}!{conflicts}{}]",
                style::Bold,
                color::Fg(color::Red),
                style::Reset
            )?;
        } else {
            write!(f, " [!{conflicts}]")?;
        }
    }

    if working_tree.any() {
        write!(f, " {}w{}[", color::Fg(color::Yellow), style::Reset)?;
        Display::fmt(working_tree, f)?;
        f.write_char(']')?;
    }

    if index.any() {
        write!(f, " {}i{}[", color::Fg(color::Green), style::Reset)?;
        Display::fmt(index, f)?;
        f.write_char(']')?;
    }

    Ok(())
}

impl Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        match self {
            Repo::Headless {
                working_tree,
                index,
            } => {
                if f.alternate() {
                    write!(
                        f,
                        "[{}{}headless{}]",
                        style::Bold,
                        color::Fg(color::Blue),
                        style::Reset
                    )?;
                } else {
                    write!(f, "[headless]")?;
                }

                fmt_changes(f, &working_tree, &index, None)?;
            }
            Repo::Clean { head } => Display::fmt(head, f)?,
            Repo::Detached {
                head,
                working_tree,
                index,
            } => {
                if f.alternate() {
                    write!(f, "{head:#7}")?;
                } else {
                    write!(f, "{head:7}")?;
                }

                fmt_changes(f, &working_tree, &index, None)?;
            }
            Repo::Working {
                branch,
                working_tree,
                index,
            } => {
                Display::fmt(branch, f)?;
                fmt_changes(f, &working_tree, &index, None)?;
            }
            Repo::Conflicted {
                kind,
                orig,
                merge,
                working_tree,
                index,
                conflicts,
            } => {
                Display::fmt(orig, f)?;

                f.write_str(match kind {
                    ConflictKind::Merge => " <- ",
                    ConflictKind::Rebase => " -> ",
                })?;

                Display::fmt(merge, f)?;

                fmt_changes(f, &working_tree, &index, Some(*conflicts))?;
            }
        }

        Ok(())
    }
}
