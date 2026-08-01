use std::fs;
use std::path::{Path, PathBuf};

use cix_cixfile::parse;

const FIXTURE_MIN: usize = 40;
const FIXTURE_MAX: usize = 65;

#[test]
fn torture_corpus_diagnostics_match_snapshots_and_quality_grades() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/torture");
    let mut fixtures = fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| panic!("reading {}: {error}", fixture_dir.display()))
        .map(|entry| entry.expect("fixture directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "cix"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(
        (FIXTURE_MIN..=FIXTURE_MAX).contains(&fixtures.len()),
        "expected {FIXTURE_MIN}–{FIXTURE_MAX} torture Cixfiles, found {}",
        fixtures.len()
    );

    for fixture in &fixtures {
        check_fixture(fixture);
    }

    let snapshot_count = fs::read_dir(fixture_dir.join("snapshots"))
        .expect("reading torture snapshots")
        .map(|entry| entry.expect("snapshot directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "snap")
        })
        .count();
    assert_eq!(
        snapshot_count,
        fixtures.len(),
        "every torture Cixfile must have exactly one committed snapshot"
    );
}

fn check_fixture(path: &Path) {
    let fixture = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let grade_line = fixture
        .lines()
        .next()
        .unwrap_or_else(|| panic!("{} is empty", path.display()));
    let grade = grade_line
        .strip_prefix("# grade: ")
        .unwrap_or_else(|| panic!("{} has no # grade header", path.display()))
        .trim_end();
    let result = parse(&fixture);
    let snapshot = if grade == "accepted" {
        result.unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
        "accepted\n".to_owned()
    } else {
        let fields = parse_grade(grade, path);
        let error = result
            .err()
            .unwrap_or_else(|| panic!("{} should produce a diagnostic", path.display()));
        assert_eq!(
            fields.line,
            error.line,
            "{} diagnostic points at the wrong physical line",
            path.display()
        );
        assert_eq!(fields.problem, "pass", "{} problem grade", path.display());
        assert_eq!(fields.fix, "pass", "{} fix grade", path.display());
        assert_eq!(
            error.message.lines().count(),
            1,
            "{} should keep one problem and one fix on one message line",
            path.display()
        );
        assert!(
            error.message.len() <= 260,
            "{} diagnostic became an essay: {} bytes",
            path.display(),
            error.message.len()
        );
        assert!(
            !contains_design_reference(&error.message),
            "{} leaks a design-journal identifier: {}",
            path.display(),
            error.message
        );
        check_doc_anchor(fields.docs, &error.message, path);
        format!("{error}\n")
    };

    let snapshot_path = path
        .parent()
        .expect("fixture parent")
        .join("snapshots")
        .join(format!(
            "{}.snap",
            path.file_stem().expect("fixture stem").to_string_lossy()
        ));
    if std::env::var_os("CIX_BLESS_DIAGNOSTICS").is_some() {
        fs::write(&snapshot_path, &snapshot)
            .unwrap_or_else(|error| panic!("writing {}: {error}", snapshot_path.display()));
    }
    let expected = fs::read_to_string(&snapshot_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", snapshot_path.display()));
    assert_eq!(expected, snapshot, "snapshot drift for {}", path.display());
}

struct Grade<'a> {
    problem: &'a str,
    line: usize,
    fix: &'a str,
    docs: &'a str,
}

fn parse_grade<'a>(grade: &'a str, path: &Path) -> Grade<'a> {
    let mut problem = None;
    let mut line = None;
    let mut fix = None;
    let mut docs = None;
    for field in grade.split_whitespace() {
        let (key, value) = field
            .split_once('=')
            .unwrap_or_else(|| panic!("{} has malformed grade field {field:?}", path.display()));
        match key {
            "problem" => problem = Some(value),
            "line" => {
                line = Some(value.parse().unwrap_or_else(|_| {
                    panic!("{} has non-numeric grade line {value:?}", path.display())
                }))
            }
            "fix" => fix = Some(value),
            "docs" => docs = Some(value),
            _ => panic!("{} has unknown grade field {key:?}", path.display()),
        }
    }
    Grade {
        problem: problem.unwrap_or_else(|| panic!("{} has no problem grade", path.display())),
        line: line.unwrap_or_else(|| panic!("{} has no line grade", path.display())),
        fix: fix.unwrap_or_else(|| panic!("{} has no fix grade", path.display())),
        docs: docs.unwrap_or_else(|| panic!("{} has no docs grade", path.display())),
    }
}

fn check_doc_anchor(reference: &str, message: &str, fixture: &Path) {
    if reference == "n/a" {
        return;
    }
    assert!(
        message.contains(reference),
        "{} is graded against {reference} but does not cite it: {message}",
        fixture.display()
    );
    let (relative, anchor) = reference
        .split_once('#')
        .unwrap_or_else(|| panic!("{} has malformed docs grade {reference}", fixture.display()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let document = fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("reading {reference}: {error}"));
    assert!(
        document.contains(&format!("id=\"{anchor}\"")),
        "{} cites missing explicit anchor {reference}",
        fixture.display()
    );
}

fn contains_design_reference(message: &str) -> bool {
    let bytes = message.as_bytes();
    bytes
        .windows(2)
        .any(|pair| pair[0] == b'D' && pair[1].is_ascii_digit())
}
