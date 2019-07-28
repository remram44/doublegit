use rusqlite::Connection;
use rusqlite::types::ToSql;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::ops::Not;
use std::path::Path;
use std::process;

use crate::git::get_sha;

fn time(n: u32) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.ymd(2019, 3, 16).and_hms(17, n, 0)
}

fn timestr(n: u32) -> String {
    format!("2019-03-16 17:{:02}:00", n)
}

fn env(n: u32) -> Vec<(String, String)> {
    let t = timestr(n);
    vec![
        ("GIT_COMMITTER_DATE".into(), t.clone()),
        ("GIT_AUTHOR_DATE".into(), t),
        ("GIT_AUTHOR_NAME".into(), "doublegit".into()),
        ("GIT_AUTHOR_EMAIL".into(), "doublegit@example.com".into()),
        ("GIT_COMMITTER_NAME".into(), "doublegit".into()),
        ("GIT_COMMITTER_EMAIL".into(), "doublegit@example.com".into()),
    ]
}

#[test]
fn test_update() {
    let test_dir = tempfile::Builder::new()
        .prefix("doublegit_test_")
        .tempdir().unwrap();

    // Set up the "remote" we'll be watching
    let origin = test_dir.path().join("origin");
    fs::create_dir(&origin).unwrap();
    assert!(process::Command::new("git")
        .arg("init")
        .current_dir(&origin)
        .status().unwrap().success());

    let write = |contents: &str| {
        let mut file = fs::File::create(origin.join("f")).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        assert!(process::Command::new("git")
                .args(&["add", "f"])
                .current_dir(&origin)
                .status().unwrap().success());
    };

    let commit = |n: u32, msg: &str| {
        let t = timestr(n);
        assert!(process::Command::new("git")
            .arg("commit")
            .arg(format!("--date={}", t))
            .arg("-m")
            .arg(msg)
            .current_dir(&origin)
            .envs(env(n))
            .status().unwrap().success());
    };

    // Set up the recording folder
    let mirror = test_dir.path().join("mirror");
    fs::create_dir(&mirror).unwrap();
    assert!(process::Command::new("git")
            .arg("init")
            .arg("--bare")
            .current_dir(&mirror)
            .status().unwrap().success());
    {
        let mut file = fs::File::create(mirror.join("config")).unwrap();
        file.write_all(b"\
            [core]\n\
            \trepositoryformatversion = 0\n\
            \tfilemode = true\n\
            \tbare = true\n\
            \tlogallrefupdates = false\n\
            [remote \"origin\"]\n\
            \turl = ../origin\n\
            \tfetch = +refs/heads/*:refs/remotes/origin/*\n"
        ).unwrap();
    }
    assert!(mirror.join("gitarchive.sqlite3").exists().not());

    // New branch 'br1'
    assert!(process::Command::new("git")
            .args(&["checkout", "-b", "br1"])
            .current_dir(&origin)
            .status().unwrap().success());
    write("one");
    commit(0, "one");
    let hash_one = "ae79568054d9fa2e4956968310655e9bcbd60e2f";
    crate::update_with_date(&mirror, time(1)).unwrap();
    assert!(mirror.join("gitarchive.sqlite3").exists());
    check_db(
        &mirror,
        &[
            ("br1", 1, None, hash_one),
        ],
        false,
    );
    check_refs(
        &mirror,
        &[hash_one],
    );

    // Update branch br1
    write("two");
    commit(2, "two");
    let hash_two = "8dcda34bbae83d2e3d856cc5dbc356ee6e947619";
    crate::update_with_date(&mirror, time(3)).unwrap();
    check_db(
        &mirror,
        &[
            ("br1", 1, Some(3), hash_one),
            ("br1", 3, None, hash_two),
        ],
        false,
    );
    check_refs(
        &mirror,
        &[hash_two],
    );

    // Force-push branch br1 back
    assert!(process::Command::new("git")
            .args(&["reset", "--keep", hash_one])
            .current_dir(&origin)
            .status().unwrap().success());
    crate::update_with_date(&mirror, time(4)).unwrap();
    check_db(
        &mirror,
        &[
            ("br1", 1, Some(3), hash_one),
            ("br1", 3, Some(4), hash_two),
            ("br1", 4, None, hash_one),
        ],
        false,
    );
    check_refs(
        &mirror,
        &[hash_two],
    );

    // Delete branch br1, create br2
    assert!(process::Command::new("git")
            .args(&["checkout", "-b", "br2"])
            .current_dir(&origin)
            .status().unwrap().success());
    assert!(process::Command::new("git")
            .args(&["branch", "-D", "br1"])
            .current_dir(&origin)
            .status().unwrap().success());
    write("three");
    commit(5, "three");
    let hash_three = "54356c0e8c1cb663294d64157f517f980e5fbd98";
    crate::update_with_date(&mirror, time(6)).unwrap();
    check_db(
        &mirror,
        &[
            ("br1", 1, Some(3), hash_one),
            ("br1", 3, Some(4), hash_two),
            ("br1", 4, Some(6), hash_one),
            ("br2", 6, None, hash_three),
        ],
        false,
    );
    check_refs(
        &mirror,
        &[
            hash_two,
            hash_three,
        ],
    );

    // Create light-weight tag1
    assert!(process::Command::new("git")
            .args(&["tag", "tag1"])
            .arg(hash_one)
            .current_dir(&origin)
            .status().unwrap().success());
    crate::update_with_date(&mirror, time(7)).unwrap();
    check_db(
        &mirror,
        &[
            ("tag1", 7, None, hash_one),
        ],
        true,
    );
    check_refs(
        &mirror,
        &[
            hash_two,
            hash_three,
        ],
    );

    // Create annotated tag2
    assert!(process::Command::new("git")
            .args(&["tag", "-a", "tag2", "-m", "tag2msg"])
            .arg(hash_two)
            .current_dir(&origin)
            .envs(env(8))
            .status().unwrap().success());
    let hash_tag2_1 = "8fda1c0cfb4957e376fba4b53bf3ce080e25300c";
    crate::update_with_date(&mirror, time(8)).unwrap();
    check_db(
        &mirror,
        &[
            ("tag1", 7, None, hash_one),
            ("tag2", 8, None, hash_tag2_1),
        ],
        true,
    );
    check_refs(
        &mirror,
        &[
            hash_three,
        ],
    );

    //    /-- two (tag2)
    // one
    //    \-- three (br2,tag1)

    // Move the tags
    assert!(process::Command::new("git")
            .args(&["tag", "-f", "tag1"])
            .arg(hash_two)
            .current_dir(&origin)
            .status().unwrap().success());
    assert!(process::Command::new("git")
            .args(&["tag", "-a", "-f", "tag2", "-m", "tag2msg"])
            .arg(hash_one)
            .current_dir(&origin)
            .envs(env(9))
            .status().unwrap().success());
    let hash_tag2_2 = "a64697beb90c35d198fd25f2985cbc9e1ac1783e";
    crate::update_with_date(&mirror, time(9)).unwrap();
    check_db(
        &mirror,
        &[
            ("tag1", 7, Some(9), hash_one),
            ("tag2", 8, Some(9), hash_tag2_1),
            ("tag1", 9, None, hash_two),
            ("tag2", 9, None, hash_tag2_2),
        ],
        true,
    );
    check_refs(
        &mirror,
        &[
            hash_two,
            hash_three,
        ],
    );

    // Remove the tags
    assert!(process::Command::new("git")
            .args(&["tag", "-d", "tag1", "tag2"])
            .current_dir(&origin)
            .status().unwrap().success());
    crate::update_with_date(&mirror, time(10)).unwrap();
    check_db(
        &mirror,
        &[
            ("tag1", 7, Some(9), hash_one),
            ("tag2", 8, Some(9), hash_tag2_1),
            ("tag1", 9, Some(10), hash_two),
            ("tag2", 9, Some(10), hash_tag2_2),
        ],
        true,
    );
    check_refs(
        &mirror,
        &[
            hash_two,
            hash_three,
        ],
    );

    // Check the non-branch refs keeping the tags alive are there
    let output = process::Command::new("git")
        .arg("show-ref")
        .current_dir(&mirror)
        .output().unwrap();
    assert!(output.status.success());
    let mut tag_refs = HashSet::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        if line.is_empty().not() {
            let line = std::str::from_utf8(&line[41..]).unwrap();
            if line.starts_with("refs/kept-tags/") {
                tag_refs.insert(line[15..].into());
            }
        }
    }
    assert_eq!(
        tag_refs,
        [hash_tag2_1, hash_tag2_2].into_iter()
            .map(|h| format!("tag-{}", h))
            .collect(),
    );
}

fn check_db(
    repo: &Path,
    expected: &[(&str, u32, Option<u32>, &str)],
    tags: bool,
) {
    // Format the expected list: make the dates from numbers
    let expected = expected.into_iter().map(
        |(name, from_date, to_date, sha)|
        (
            name.to_string(),
            timestr(*from_date),
            to_date.map(timestr),
            sha.to_string(),
        )
    ).collect::<Vec<_>>();

    // Get the actual list from the database
    let conn = Connection::open(repo.join("gitarchive.sqlite3")).unwrap();
    let mut stmt = conn.prepare(
        "
        SELECT name, from_date, to_date, sha
        FROM refs
        WHERE tag=?
        ORDER BY from_date, name;
        "
    ).unwrap();
    let refs: Vec<_> = stmt.query_map(
        &[&tags as &ToSql],
        |row| (
            row.get::<_, String>(0),
            row.get::<_, String>(1),
            row.get::<_, Option<String>>(2),
            row.get::<_, String>(3),
        ),
    ).unwrap().map(Result::unwrap).collect();

    // Assert
    assert_eq!(refs, expected);
}

fn check_refs(repo: &Path, expected: &[&str]) {
    // Format the expected list (add 'keep-' prefix)
    let expected = expected.into_iter()
        .map(|h| format!("keep-{}", h))
        .collect();

    // Get the actual list from Git
    let output = process::Command::new("git")
            .arg("branch")
            .current_dir(&repo)
            .output().unwrap();
    assert!(output.status.success());
    let mut refs = HashSet::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        let line = std::str::from_utf8(line).unwrap().trim();
        if line.is_empty().not() {
            refs.insert(line.into());

            // Check that 'keep-abc' points to 'abc'
            if line.starts_with("keep-") {
                assert_eq!(&line[5..], get_sha(repo, line).unwrap());
            }
        }
    }

    // Assert
    assert_eq!(refs, expected);
}
