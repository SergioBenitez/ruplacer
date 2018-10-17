use colored::*;
use difference::{Changeset, Difference};
use std;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use line_patcher::LinePatcher;
use query::Query;

pub struct FilePatcher {
    replacements: Vec<Replacement>,
    path: PathBuf,
    new_contents: String,
}

impl FilePatcher {
    pub fn new(path: PathBuf, query: &Query) -> Result<FilePatcher, std::io::Error> {
        let mut replacements = vec![];
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut new_contents = String::new();
        for (num, chunk) in reader.split('\n' as u8).enumerate() {
            let chunk = chunk?; // consume the io::error
            let line = String::from_utf8(chunk);
            if line.is_err() {
                let io_error: std::io::Error = std::io::ErrorKind::InvalidData.into();
                return Err(io_error);
            }
            let line = line.unwrap();
            let line_patcher = LinePatcher::new(&line);
            let new_line = line_patcher.replace(&query);
            if new_line != line {
                let replacement = Replacement {
                    line_no: num + 1,
                    old: line,
                    new: new_line.clone(),
                };
                replacements.push(replacement);
                new_contents.push_str(&new_line);
            } else {
                new_contents.push_str(&line);
            }
            new_contents.push_str("\n");
        }
        Ok(FilePatcher {
            replacements,
            path,
            new_contents,
        })
    }

    pub fn replacements(&self) -> &Vec<Replacement> {
        &self.replacements
    }

    pub fn run(&self) -> Result<(), std::io::Error> {
        std::fs::write(&self.path, &self.new_contents)?;
        Ok(())
    }

    pub fn print_patch(&self) {
        println!(
            "{} {}",
            "Patching".blue(),
            self.path.to_string_lossy().bold()
        );
        for replacement in &self.replacements {
            replacement.print_self();
            print!("\n");
        }
        print!("\n");
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Replacement {
    line_no: usize,
    old: String,
    new: String,
}

impl Replacement {
    fn print_self(&self) {
        let changeset = Changeset::new(&self.old, &self.new, "");
        let differences = changeset.diffs;

        let mut add_changes = vec![];
        let mut rem_changes = vec![];

        for difference in differences {
            let diff = Diff::from_difference(&difference);
            match diff.diff_type {
                DiffType::Same => {
                    add_changes.push(diff.clone());
                    rem_changes.push(diff.clone());
                }
                DiffType::Rem => {
                    rem_changes.push(diff);
                }
                DiffType::Add => {
                    add_changes.push(diff);
                }
            }
        }
        print!("{} ", "--".red());
        let compact_rem = compact_changeset(&rem_changes);
        for diff in compact_rem {
            diff.print_self();
        }

        print!("\n");
        let compact_add = compact_changeset(&add_changes);
        print!("{} ", "++".green());
        for diff in compact_add {
            diff.print_self();
        }
    }
}

// The difference::Difference type is hard to work with,
// and we cannot implement any trait on it, so use our own
// little struct intsead, and implement clone()
#[derive(Debug, PartialEq, Clone)]
enum DiffType {
    Rem,
    Same,
    Add,
}

#[derive(Debug, PartialEq, Clone)]
struct Diff {
    diff_type: DiffType, // this makes checking type of diff a lot easier
    text: String,
}

impl Diff {
    fn from_difference(difference: &Difference) -> Diff {
        match difference {
            Difference::Add(s) => Self::new_add(s),
            Difference::Rem(s) => Self::new_rem(s),
            Difference::Same(s) => Self::new_same(s),
        }
    }

    fn new_add(text: &str) -> Diff {
        Diff {
            diff_type: DiffType::Add,
            text: text.to_string(),
        }
    }

    fn new_same(text: &str) -> Diff {
        Diff {
            diff_type: DiffType::Same,
            text: text.to_string(),
        }
    }

    fn new_rem(text: &str) -> Diff {
        Diff {
            diff_type: DiffType::Rem,
            text: text.to_string(),
        }
    }

    fn print_self(self) {
        let desc = match self.diff_type {
            DiffType::Same => self.text.normal(),
            DiffType::Rem => self.text.red().underline(),
            DiffType::Add => self.text.green().underline(),
        };
        print!("{}", desc)
    }
}

fn is_add_sandwich(a: &Diff, b: &Diff, c: &Diff) -> bool {
    a.diff_type == DiffType::Add && b.diff_type == DiffType::Same && c.diff_type == DiffType::Add
}

fn is_rem_sandwich(a: &Diff, b: &Diff, c: &Diff) -> bool {
    a.diff_type == DiffType::Rem && b.diff_type == DiffType::Same && c.diff_type == DiffType::Rem
}

fn squash(changeset: &Vec<Diff>, index: usize) -> Option<Diff> {
    let n = changeset.len();
    if index < 1 {
        return None;
    }
    if index >= (n - 1) {
        return None;
    }
    let current = &changeset[index];
    let previous = &changeset[index - 1];
    let next = &changeset[index + 1];
    let squashed_text = previous.text.to_string() + &current.text + &next.text;
    if is_add_sandwich(&previous, &current, &next) {
        return Some(Diff::new_add(&squashed_text));
    }
    if is_rem_sandwich(&previous, &current, &next) {
        return Some(Diff::new_rem(&squashed_text));
    }
    None
}

fn compact_changeset(changeset: &Vec<Diff>) -> Vec<Diff> {
    let mut res = vec![];
    let n = changeset.len();
    let mut skip_next = false;
    for i in 0..n {
        let current_diff = &changeset[i];
        let squashed = squash(&changeset, i);
        if current_diff.text.len() < 3 {
            if let Some(squash) = squashed {
                // replace last 'add' or 'rm' chunk
                res[i - 1] = squash;
                // skip next chunk
                skip_next = true;
            } else {
                // same is too long, do nothing
            }
        } else {
            if !skip_next {
                res.push(current_diff.clone());
            }
            skip_next = false;
        }
    }
    res
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use super::*;
    use query;
    use std::fs;

    #[test]
    fn test_compact_diff_empty() {
        let change = vec![];
        let actual = compact_changeset(&change);
        assert_eq!(actual, vec![]);
    }

    #[test]
    fn test_compact_diff_just_one() {
        let change = vec![Diff::new_add("some stuff")];
        let actual = compact_changeset(&change);
        assert_eq!(actual, vec![Diff::new_add("some stuff")]);
    }

    #[test]
    fn test_compact_diff_just_two() {
        let change = vec![Diff::new_add("one"), Diff::new_rem("two")];
        let actual = compact_changeset(&change);
        assert_eq!(actual, vec![Diff::new_add("one"), Diff::new_rem("two")]);
    }

    #[test]
    fn test_compact_diff_replace_small_same_with_previous_and_next_add() {
        let change = vec![
            Diff::new_add("one"),
            Diff::new_same("_"),
            Diff::new_add("two"),
        ];
        let actual = compact_changeset(&change);
        assert_eq!(actual, vec![Diff::new_add("one_two")]);
    }

    #[test]
    fn test_compact_diff_do_not_replace_big_same() {
        let change = vec![
            Diff::new_add("one "),
            Diff::new_same(" - and this is - "),
            Diff::new_add(" three"),
        ];
        let actual = compact_changeset(&change);
        assert_eq!(
            actual,
            vec![
                Diff::new_add("one "),
                Diff::new_same(" - and this is - "),
                Diff::new_add(" three"),
            ]
        );
    }

    #[test]
    fn test_compact_diff_do_not_keep_skipping() {
        let change = vec![
            Diff::new_add("one"),
            Diff::new_same("_"),
            Diff::new_add("two"),
            Diff::new_add("three"),
            Diff::new_same(" - and - "),
            Diff::new_add("four"),
        ];
        let actual = compact_changeset(&change);
        assert_eq!(
            actual,
            vec![
                Diff::new_add("one_two"),
                Diff::new_add("three"),
                Diff::new_same(" - and - "),
                Diff::new_add("four"),
            ]
        );
    }

    #[test]
    fn test_compact_diff_rem_sandwich() {
        let change = vec![
            Diff::new_rem("one"),
            Diff::new_same("_"),
            Diff::new_rem("two"),
        ];
        let actual = compact_changeset(&change);
        assert_eq!(actual, vec![Diff::new_rem("one_two")]);
    }

    #[test]
    fn test_compact_diff_two_sandwhiches() {
        let change = vec![
            Diff::new_add("one"),
            Diff::new_same("_"),
            Diff::new_add("two"),
            Diff::new_same("_"),
            Diff::new_add("three"),
        ];

        let actual = compact_changeset(&change);
        assert_eq!(actual, vec![Diff::new_add("one_two_three")]);
    }

    #[test]
    fn test_compute_replacements() {
        let top_path = std::path::Path::new("tests/data/top.txt");
        let file_patcher =
            FilePatcher::new(top_path.to_path_buf(), &query::substring("old", "new")).unwrap();
        let replacements = file_patcher.replacements();
        assert_eq!(replacements.len(), 1);
        let actual_replacement = &replacements[0];
        assert_eq!(actual_replacement.line_no, 2);
        // ruplacer preserves line endings: on Windows, there is a
        // possibility the actual lines contain \r, depending
        // of the git configuration.
        // So strip the \r before comparing them to the expected result.
        let actual_new = actual_replacement.new.replace("\r", "");
        let actual_old = actual_replacement.old.replace("\r", "");
        assert_eq!(actual_new, "Top: new is nice");
        assert_eq!(actual_old, "Top: old is nice");
    }

    #[test]
    fn test_patch_file() {
        let temp_dir = tempdir::TempDir::new("test-ruplacer").unwrap();
        let file_path = temp_dir.path().join("foo.txt");
        fs::write(&file_path, "first line\nI say: old is nice\nlast line\n").unwrap();
        let file_patcher =
            FilePatcher::new(file_path.to_path_buf(), &query::substring("old", "new")).unwrap();
        file_patcher.run().unwrap();
        let actual = fs::read_to_string(&file_path).unwrap();
        let expected = "first line\nI say: new is nice\nlast line\n";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_replacement_display() {
        // This test cannot fail. It's just here so you can tweak the look and feel
        // of ruplacer easily.
        let replacement = Replacement {
            line_no: 1,
            old: "trustchain_creation: 0".to_owned(),
            new: "blockchain_creation: 0".to_owned(),
        };
        replacement.print_self();
    }
}
