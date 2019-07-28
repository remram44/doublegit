use regex::Regex;
use std::collections::HashSet;
use std::ops::Not;
use std::path::Path;
use std::process;

use crate::{Error, Operation, Ref};

fn parse_operation(chr: &str) -> Result<Operation, Error> {
    if chr.len() != 1 {
        return Err(Error::git("Parse error: invalid operation"));
    }
    let chr = chr.as_bytes()[0];
    Ok(match chr {
        b' ' => Operation::FastForward,
        b'+' => Operation::Forced,
        b'-' => Operation::Pruned,
        b't' => Operation::Tag,
        b'*' => Operation::New,
        b'!' => Operation::Reject,
        b'=' => Operation::Noop,
        _ => return Err(Error::git("Parse error: invalid operation")),
    })
}

pub struct FetchOutput {
    pub new: HashSet<Ref>,
    pub changed: HashSet<Ref>,
    pub removed: HashSet<Ref>,
}

pub fn fetch(repository: &Path) -> Result<FetchOutput, Error> {
    let output = process::Command::new("git")
        .args(&["fetch", "--prune", "origin",
                "+refs/tags/*:refs/tags/*",
                "+refs/heads/*:refs/remotes/origin/*"])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(format!("`git fetch` returned {}",
                                      output.status)));
    }
    parse_fetch_output(&output.stderr)
}

fn parse_fetch_output(output: &[u8]) -> Result<FetchOutput, Error> {
    lazy_static! {
        static ref _RE_FETCH: Regex = Regex::new(
            r"^ ([+t*! -]) +([^ ]+|\[[^\]]+\]) +([^ ]+) +-> +([^ ]+)(?: +(.+))?$"
        ).unwrap();
    }
    let remote = "origin";
    let mut new = HashSet::new();
    let mut changed = HashSet::new();
    let mut removed = HashSet::new();
    for line in output.split(|&b| b == b'\n') {
        let line = std::str::from_utf8(line)
            .map_err(|_| Error::git("Non-utf8 branch name"))?;
        if let Some(m) = _RE_FETCH.captures(line) {
            info!("> {}", line);
            let op = m.get(1).map_or("", |m| m.as_str());
            let summary = m.get(2).map_or("", |m| m.as_str());
            let from = m.get(3).map_or("", |m| m.as_str());
            let to = m.get(4).map_or("", |m| m.as_str());
            let reason = m.get(5).map_or("", |m| m.as_str());

            let op = parse_operation(op)?;
            match op {
                Operation::New => {
                    if !to.contains('/') { // tag
                        let ref_ = Ref {
                            remote: remote.into(),
                            name: to.into(),
                            tag: true,
                        };
                        info!("New tag {}", ref_.name);
                        new.insert(ref_);
                    } else {
                        let ref_ = Ref::parse_remote_ref(to, remote)?;
                        info!("New branch {}", ref_.name);
                        new.insert(ref_);
                    }
                }
                Operation::FastForward|Operation::Forced => {
                    let ref_ = Ref::parse_remote_ref(to, remote)?;
                    info!("Updated branch {}", ref_.name);
                    changed.insert(ref_);
                }
                Operation::Pruned => {
                    if !to.contains('/') { // tag
                        let ref_ = Ref {
                            remote: remote.into(),
                            name: to.into(),
                            tag: true,
                        };
                        info!("Pruned tag {}", ref_.name);
                        removed.insert(ref_);
                    } else {
                        let ref_ = Ref::parse_remote_ref(to, remote)?;
                        info!("Pruned branch {}", ref_.name);
                        removed.insert(ref_);
                    }
                }
                Operation::Tag => {
                    let ref_ = Ref {
                        remote: remote.into(),
                        name: to.into(),
                        tag: true,
                    };
                    info!("Updated tag {}", ref_.name);
                    changed.insert(ref_);
                }
                Operation::Reject => {
                    return Err(Error::Git(format!("Error updating ref {}",
                                                  to)));
                }
                Operation::Noop => {}
            }
        } else {
            info!("! {}", line);
        }
    }
    Ok(FetchOutput { new, changed, removed })
}

pub fn get_sha(repository: &Path, refname: &str) -> Result<String, Error> {
    let output = process::Command::new("git")
        .args(&["rev-parse", refname])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .stderr(process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(format!("`git rev-parse` returned {}",
                                      output.status)));
    }
    let sha = std::str::from_utf8(&output.stdout)
        .map_err(|_| Error::git("Non-utf8 sha?!"))?;
    Ok(sha.trim().into())
}

pub fn make_branch(
    repository: &Path,
    name: &str,
    sha: &str,
) -> Result<(), Error> {
    let status = process::Command::new("git")
        .args(&["branch", "-f", name, sha])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(Error::Git(format!("`git branch -f` returned {}", status)));
    }
    Ok(())
}

pub fn make_ref(
    repository: &Path,
    name: &str,
    sha: &str,
) -> Result<(), Error> {
    let status = process::Command::new("git")
        .args(&["update-ref", name, sha])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(Error::Git(
            format!("`git update-ref` returned {}", status)
        ));
    }
    Ok(())
}

pub fn is_annotated_tag(
    repository: &Path,
    target: &str,
) -> Result<bool, Error> {
    let output = process::Command::new("git")
        .args(&["cat-file", "-t", target])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .stderr(process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(format!("`git cat-file -t` returned {}",
                                      output.status)));
    }
    Ok(output.stdout == b"tag\n")
}

pub fn included_branches(
    repository: &Path, target: &str,
) -> Result<Vec<String>, Error> {
    let output = process::Command::new("git")
        .args(&["branch", "--merged", target])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .stderr(process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(format!("`git branch --merged` returned {}",
                                      output.status)));
    }
    let mut refs = Vec::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        let line = std::str::from_utf8(line)
            .map_err(|_| Error::git("Non-utf8 branch name"))?.trim();
        if line.is_empty().not() {
            refs.push(line.into());
        }
    }
    Ok(refs)
}

pub fn including_branches(
    repository: &Path,
    target: &str,
) -> Result<Vec<String>, Error> {
    let output = process::Command::new("git")
        .args(&["branch", "--contains", target])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .stderr(process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(format!("`git branch --contains` returned {}",
                                      output.status)));
    }
    let mut refs = Vec::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        let line = std::str::from_utf8(line)
            .map_err(|_| Error::git("Non-utf8 branch name"))?.trim();
        if line.is_empty().not() {
            refs.push(line.into());
        }
    }
    Ok(refs)
}

pub fn delete_branch(repository: &Path, name: &str) -> Result<(), Error> {
    let status = process::Command::new("git")
        .args(&["branch", "-D", name])
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(Error::Git(format!("`git branch -D` returned {}", status)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use rusqlite::types::ToSql;
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use std::ops::Not;
    use std::path::Path;
    use std::process;

    use crate::{Operation, Ref};
    use crate::git::{get_sha, parse_operation, parse_fetch_output};

    #[test]
    fn test_parse_operation() {
        assert!(parse_operation("").is_err());
        assert!(parse_operation("++").is_err());
        assert_eq!("\u{E9}".len(), 2);
        assert!(parse_operation("\u{E9}").is_err());
        assert_eq!(parse_operation("+").unwrap(), Operation::Forced);
    }

    #[test]
    fn test_parse_fetch() {
        let stderr: &[u8] = b"
Fetching origin
remote: Enumerating objects: 14, done.
remote: Counting objects: 100% (14/14), done.
remote: Compressing objects: 100% (11/11), done.
remote: Total 14 (delta 3), reused 12 (delta 1), pack-reused 0
Unpacking objects: 100% (14/14), done.
From github.com:remram44/doublegit
 * [new branch]      master     -> origin/master
   673b728..466e90b  devel      -> origin/devel
 - [deleted]         (none)     -> origin/old
";
        let output = parse_fetch_output(stderr).unwrap();
        assert_eq!(
            output.new,
            [
                Ref {
                    remote: "origin".into(),
                    name: "master".into(),
                    tag: false,
                },
            ].iter().cloned().collect(),
        );
        assert_eq!(
            output.changed,
            [
                Ref {
                    remote: "origin".into(),
                    name: "devel".into(),
                    tag: false,
                },
            ].iter().cloned().collect(),
        );
        assert_eq!(
            output.removed,
            [
                Ref {
                    remote: "origin".into(),
                    name: "old".into(),
                    tag: false,
                },
            ].iter().cloned().collect(),
        );
    }

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
        crate::update_with_date(&mirror, time(10));
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
            |row| Ok((
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, Option<String>>(2).unwrap(),
                row.get::<_, String>(3).unwrap(),
            )),
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
}
