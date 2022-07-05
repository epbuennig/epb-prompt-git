// use crate::repo;
use std::{
    borrow::Cow,
    fs::File,
    io::{self, Read},
    path::Path,
};

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

// ignore non `N...` (submodules)
// <prefix> <XY> N... <...>
pub fn parse_xy_line(line: &str, prefix: &str) -> Option<(char, char)> {
    line.strip_prefix(prefix)
        .and_then(|rest| (&rest[3..7] == "N...").then(|| &rest[..2]))
        .map(|xy| (xy.as_bytes()[0] as char, xy.as_bytes()[1] as char))
}

pub fn try_get_file_content(path: impl AsRef<Path>) -> io::Result<Option<String>> {
    match File::open(path) {
        Ok(mut file) => {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            assert_eq!(content.pop(), Some('\n'));
            Ok(Some(content))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}
