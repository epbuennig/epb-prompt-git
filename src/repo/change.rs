use std::{
    array,
    fmt::{Debug, Display},
    iter::Enumerate,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    Add = 0,
    Mod = 1,
    Del = 2,
    Ren = 3,
    Typ = 4,
}

impl Change {
    fn from_idx(value: usize) -> Self {
        match value {
            0 => Self::Add,
            1 => Self::Mod,
            2 => Self::Del,
            3 => Self::Ren,
            4 => Self::Typ,
            x => unreachable!("invalid index, expected 0..=4, got {x}"),
        }
    }

    fn fmt_with(&self, value: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use termion::{color, style};

        if f.alternate() {
            match self {
                Change::Add => write!(f, "{}+{value}{}", color::Fg(color::Green), style::Reset),
                Change::Mod => write!(f, "{}~{value}{}", color::Fg(color::Yellow), style::Reset),
                Change::Del => write!(f, "{}-{value}{}", color::Fg(color::Red), style::Reset),
                Change::Ren => write!(f, "{}*{value}{}", color::Fg(color::Cyan), style::Reset),
                Change::Typ => write!(f, "{}?{value}{}", color::Fg(color::Magenta), style::Reset),
            }
        } else {
            write!(
                f,
                "{}{value}",
                match self {
                    Change::Add => '+',
                    Change::Mod => '~',
                    Change::Del => '-',
                    Change::Ren => '*',
                    Change::Typ => '?',
                }
            )
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Changes([usize; 5]);

impl Changes {
    pub fn new() -> Self {
        Self([0; 5])
    }

    pub fn any(&self) -> bool {
        self.iter().any(|(_, &v)| v != 0)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter().enumerate())
    }
}

impl Debug for Changes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Changes")
            .field("add", &self[Change::Add])
            .field("mod", &self[Change::Mod])
            .field("del", &self[Change::Del])
            .field("ren", &self[Change::Ren])
            .finish()
    }
}

impl Display for Changes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (change, &count) in self.iter().filter(|&(_, &v)| v != 0) {
            change.fmt_with(count, f)?;
        }

        Ok(())
    }
}

impl Index<Change> for Changes {
    type Output = usize;

    fn index(&self, index: Change) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<Change> for Changes {
    fn index_mut(&mut self, index: Change) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

pub struct Iter<'a>(Enumerate<slice::Iter<'a, usize>>);
pub struct IntoIter(Enumerate<array::IntoIter<usize, 5>>);

impl<'a> Iterator for Iter<'a> {
    type Item = (Change, &'a usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(i, v)| (Change::from_idx(i), v))
    }
}

impl Iterator for IntoIter {
    type Item = (Change, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(i, v)| (Change::from_idx(i), v))
    }
}

impl IntoIterator for Changes {
    type Item = (Change, usize);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter().enumerate())
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = (Change, &'a usize);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
