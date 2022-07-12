use std::fmt::{Debug, Display};

#[derive(Clone, PartialEq, Eq)]
pub struct RemoteBranch(String, String);

impl RemoteBranch {
    pub fn new(remote: String, branch: String) -> Self {
        Self(remote, branch)
    }
}

impl Debug for RemoteBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "refs/remote/{}/{}", self.0, self.1)
    }
}

impl Display for RemoteBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        if f.alternate() {
            write!(
                f,
                "{fg}{}{r}/{fg}{}{r}",
                self.0,
                // sparse printing
                if f.sign_aware_zero_pad() {
                    "~"
                } else {
                    &self.1
                },
                fg = color::Fg(color::Blue),
                r = style::Reset
            )
        } else {
            write!(
                f,
                "{}/{}",
                self.0,
                // sparse printing
                if f.sign_aware_zero_pad() {
                    "~"
                } else {
                    &self.1
                }
            )
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Divergence(usize, usize);

impl Divergence {
    pub fn new(ahead: usize, behind: usize) -> Self {
        debug_assert!(
            ahead != 0 || behind != 0,
            "at least one of ahead or behind should be non zero"
        );

        Self(ahead, behind)
    }

    pub fn ahead_behind(self) -> (usize, usize) {
        (self.0, self.1)
    }
}

impl Debug for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Divergence")
            .field("ahead", &self.0)
            .field("behind", &self.1)
            .finish()
    }
}

impl Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        let (ahead, behind) = self.ahead_behind();

        if f.alternate() {
            if self.0 != 0 {
                write!(
                    f,
                    "{fg}{r}{ahead}",
                    fg = color::Fg(color::Red),
                    r = style::Reset
                )?;
            }

            if self.1 != 0 {
                write!(
                    f,
                    "{fg}{r}{behind}",
                    fg = color::Fg(color::Red),
                    r = style::Reset
                )?;
            }
        } else {
            if self.0 != 0 {
                write!(f, "{ahead}")?;
            }

            if self.1 != 0 {
                write!(f, "{behind}")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Branch {
    local: String,
    remote: Option<(RemoteBranch, Option<Divergence>)>,
}

impl Debug for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (ahead, behind) = self
            .divergence()
            .map(Divergence::ahead_behind)
            .unwrap_or_default();

        f.debug_struct("Branch")
            .field("local", &self.local)
            .field("remote", &self.remote())
            .field("ahead", &ahead)
            .field("behind", &behind)
            .finish()
    }
}

impl Branch {
    pub fn new(local: String, remote_diverge: Option<(RemoteBranch, Option<Divergence>)>) -> Self {
        Self {
            local,
            remote: remote_diverge,
        }
    }

    pub fn remote(&self) -> Option<&RemoteBranch> {
        self.remote.as_ref().map(|&(ref r, _)| r)
    }

    pub fn divergence(&self) -> Option<Divergence> {
        self.remote.as_ref().map(|&(_, d)| d).flatten()
    }
}

impl Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        match self.remote() {
            Some(remote) => {
                let divergence = self.divergence();

                if f.alternate() {
                    write!(f, "{:#}", self.local)?;
                } else {
                    write!(f, "{}", self.local)?;
                }

                // sparse printing
                if f.sign_aware_zero_pad() {
                    return Ok(());
                }

                match (f.alternate(), remote.1 == self.local) {
                    (true, false) => write!(f, "[{remote:#}]")?,
                    (true, true) => write!(f, "[{remote:#0}]")?,
                    (false, false) => write!(f, "[{remote:}]")?,
                    (false, true) => write!(f, "[{remote:0}]")?,
                }

                match (f.alternate(), divergence) {
                    (true, None) => write!(f, "[{}{}]", color::Fg(color::Green), style::Reset)?,
                    (true, Some(divergence)) => write!(f, "[{divergence:#}]")?,
                    (false, None) => f.write_str("[]")?,
                    (false, Some(divergence)) => write!(f, "[{divergence}]")?,
                }
            }
            None => {
                if f.alternate() {
                    write!(f, "{:#}", self.local)?;
                } else {
                    write!(f, "{}", self.local)?;
                }

                // sparse printing
                if f.sign_aware_zero_pad() {
                    return Ok(());
                }
                if f.alternate() {
                    write!(f, "[{}-{}]", color::Fg(color::Blue), style::Reset)?;
                } else {
                    f.write_str("[-]")?;
                }
            }
        }

        Ok(())
    }
}
