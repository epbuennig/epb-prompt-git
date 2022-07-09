use std::{
    fmt::{Debug, Display, Write},
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
pub enum ConflictRef {
    Commit(Commit),
    Branch(Branch),
}

impl ConflictRef {
    pub fn commit(hash: String) -> Self {
        Self::Commit(Commit::new(hash))
    }

    pub fn branch(local: String) -> Self {
        Self::Branch(Branch::new(local, None))
    }
}

impl Display for ConflictRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictRef::Commit(commit) => Display::fmt(commit, f),
            ConflictRef::Branch(branch) => {
                // use spare flag to show no remote info on conflict
                if f.alternate() {
                    write!(f, "{:#0}", branch)
                } else {
                    write!(f, "{:0}", branch)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag(String);

impl Tag {
    pub fn new(tag: String) -> Self {
        Self(tag)
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        if f.alternate() {
            write!(
                f,
                "[{}{}{}{}]",
                style::Bold,
                color::Fg(color::Yellow),
                self.0,
                style::Reset
            )
        } else {
            write!(f, "[{}]", self.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetachedRef {
    Commit(Commit),
    Tag(Tag),
}

impl DetachedRef {
    pub fn commit(hash: String) -> Self {
        Self::Commit(Commit::new(hash))
    }

    pub fn tag(tag: String) -> Self {
        Self::Tag(Tag::new(tag))
    }
}

impl Display for DetachedRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetachedRef::Commit(commit) => Display::fmt(commit, f),
            DetachedRef::Tag(tag) => Display::fmt(tag, f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    Headless {
        working_tree: Changes,
        index: Changes,
        stash: usize,
    },
    Clean {
        head: Branch,
        stash: usize,
    },
    Detached {
        head: DetachedRef,
        working_tree: Changes,
        index: Changes,
        stash: usize,
    },
    Working {
        branch: Branch,
        working_tree: Changes,
        index: Changes,
        stash: usize,
    },
    Conflicted {
        kind: ConflictKind,
        source: ConflictRef,
        target: ConflictRef,
        working_tree: Changes,
        index: Changes,
        conflicts: usize,
        stash: usize,
    },
}

impl Prompt {
    pub fn headless(working_tree: Changes, index: Changes, stash: usize) -> Self {
        Self::Headless {
            working_tree,
            index,
            stash,
        }
    }

    pub fn clean(branch: Branch, stash: usize) -> Self {
        Self::Clean {
            head: branch,
            stash,
        }
    }

    pub fn detached(
        head: DetachedRef,
        working_tree: Changes,
        index: Changes,
        stash: usize,
    ) -> Self {
        Self::Detached {
            head,
            working_tree,
            index,
            stash,
        }
    }

    pub fn working(branch: Branch, working_tree: Changes, index: Changes, stash: usize) -> Self {
        Self::Working {
            branch,
            working_tree,
            index,
            stash,
        }
    }

    pub fn conflict(
        kind: ConflictKind,
        source: ConflictRef,
        target: ConflictRef,
        working_tree: Changes,
        index: Changes,
        conflicts: usize,
        stash: usize,
    ) -> Self {
        Self::Conflicted {
            kind,
            source,
            target,
            working_tree,
            index,
            conflicts,
            stash,
        }
    }
}

fn fmt_stash(f: &mut std::fmt::Formatter<'_>, stash: usize) -> std::fmt::Result {
    use termion::{color, style};

    if stash != 0 {
        if f.alternate() {
            write!(
                f,
                " :: {}s{}[{}]",
                color::Fg(color::Magenta),
                style::Reset,
                stash
            )?;
        } else {
            write!(f, " :: s[{}]", stash)?;
        }
    }

    Ok(())
}

fn fmt_changes(
    f: &mut std::fmt::Formatter<'_>,
    working_tree: &Changes,
    index: &Changes,
    conflicts: usize,
) -> std::fmt::Result {
    use termion::{color, style};

    if working_tree.any() || index.any() || conflicts != 0 {
        f.write_str(" ::")?;
    }

    if conflicts != 0 {
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

impl Display for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        match self {
            Prompt::Headless {
                working_tree,
                index,
                stash,
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

                fmt_stash(f, *stash)?;
                fmt_changes(f, &working_tree, &index, 0)?;
            }
            Prompt::Clean { head, stash } => {
                Display::fmt(head, f)?;
                fmt_stash(f, *stash)?;
            }
            Prompt::Detached {
                head,
                working_tree,
                index,
                stash,
            } => {
                if f.alternate() {
                    write!(f, "{head:#7}")?;
                } else {
                    write!(f, "{head:7}")?;
                }

                fmt_stash(f, *stash)?;
                fmt_changes(f, &working_tree, &index, 0)?;
            }
            Prompt::Working {
                branch,
                working_tree,
                index,
                stash,
            } => {
                Display::fmt(branch, f)?;
                fmt_stash(f, *stash)?;
                fmt_changes(f, &working_tree, &index, 0)?;
            }
            Prompt::Conflicted {
                kind,
                source,
                target,
                working_tree,
                index,
                conflicts,
                stash,
            } => {
                match kind {
                    ConflictKind::Merge => {
                        Display::fmt(source, f)?;
                        f.write_str(" <- ")?;
                        Display::fmt(target, f)?;
                    }
                    ConflictKind::Rebase => {
                        Display::fmt(target, f)?;
                        f.write_str(" -> ")?;
                        Display::fmt(source, f)?;
                    }
                }

                fmt_stash(f, *stash)?;
                fmt_changes(f, &working_tree, &index, *conflicts)?;
            }
        }

        Ok(())
    }
}
