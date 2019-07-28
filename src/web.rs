use handlebars::Handlebars;
use http::StatusCode;
use hyper::Body;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use warp::{self, Filter};
use warp::path;
use warp::reply::{Reply, Response};

use crate::Error;

pub fn serve(
    repository: &Path,
    host: std::net::IpAddr,
    port: u16,
) -> Result<(), Error> {
    // Connect to database
    let db_path = repository.join("gitarchive.sqlite3");
    let db = Connection::open(db_path)?;
    let db = Arc::new(Mutex::new(db));
    let db = warp::any().map(move || db.clone());

    // Repository path
    let repo_path = Arc::new(repository.to_path_buf());
    let repo_path = warp::any().map(move || repo_path.clone());

    // Load templates
    let mut templates = Handlebars::new();
    templates.register_template_string(
        "browse.html",
        include_str!("browse.html"),
    ).unwrap();
    let templates = Arc::new(templates);
    let templates = warp::any().map(move || templates.clone());

    let routes =
        // Index, redirects to a branch in the latest snapshot
        path::end()
            .and(db.clone()).and_then(index)
        // Repo alone ("_"), same as index
        .or(path!("_").and(path::end())
            .and(db.clone()).and_then(index))
        // Snapshot without branch, redirect to a branch
        .or(path!("_" / String).and(path::end())
            .and(db.clone()).and_then(snapshot))
        // Browse view, shows a branch in a snapshot
        .or(path!("_" / String / String).and(path::end())
            .and(db).and(repo_path).and(templates).and_then(browse));

    println!("\n    Starting server on {}:{}\n", host, port);
    warp::serve(routes).run((host, port));

    Ok(())
}

/// Redirects to main branch in latest snapshot
fn index(
    db: Arc<Mutex<Connection>>
) -> Result<Response, warp::reject::Rejection> {
    let db = db.lock().unwrap();

    // First we have to find a suitable branch
    let head = (|| -> Result<String, rusqlite::Error> {
        // If "master" exists, use that
        let mut stmt = db.prepare(
            "
            SELECT name FROM refs
            WHERE name='master' AND tag=0 AND to_date IS NULL;
            "
        )?;
        let mut rows = stmt.query(rusqlite::NO_PARAMS)?;
        if rows.next().is_some() {
            Ok("master".into())
        } else {
            // Otherwise, use whatever branch was last updated
            let mut stmt = db.prepare(
                "
                SELECT name FROM refs
                WHERE tag=0 AND to_date IS NULL
                ORDER BY from_date DESC, name DESC;
                "
            )?;
            let mut rows = stmt.query(rusqlite::NO_PARAMS)?;
            if let Some(row) = rows.next() {
                Ok(row?.get(0))
            } else {
                panic!()
            }
        }
    })().map_err(warp::reject::custom)?;
    info!("Redirecting to main branch: {}", head);

    // Redirect
    http::response::Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", format!("/_/latest/{}", head))
        .body(Body::empty()).map_err(warp::reject::custom)
}

/// Redirects to main branch in given snapshot
fn snapshot(
    date: String,
    db: Arc<Mutex<Connection>>
) -> Result<Response, warp::reject::Rejection> {
    let date = match percent_encoding::percent_decode(date.as_bytes())
        .decode_utf8()
    {
        Ok(s) => s,
        Err(_) => return Err(warp::reject::not_found()),
    };

    let db = db.lock().unwrap();

    // First we have to find the main branch
    let head = (|| -> Result<String, rusqlite::Error> {
        // If "master" exists, use that
        let mut stmt = db.prepare(
            "
            SELECT name FROM refs
            WHERE name='master' AND tag=0
                AND from_date <= ?
                AND (to_date IS NULL OR to_date > ?);
            "
        )?;
        let mut rows = stmt.query(&[&date, &date])?;
        if rows.next().is_some() {
            Ok("master".into())
        } else {
            // Otherwise, use whatever branch was last updated
            let mut stmt = db.prepare(
                "
                SELECT name FROM refs
                WHERE tag=0
                    AND from_date <= ?
                    AND (to_date IS NULL OR to_date >?)
                ORDER BY from_date DESC, name DESC;
                "
            )?;
            let mut rows = stmt.query(&[&date, &date])?;
            if let Some(row) = rows.next() {
                Ok(row?.get(0))
            } else {
                panic!()
            }
        }
    })().map_err(warp::reject::custom)?;
    info!("Redirecting to main branch at {}: {}", date, head);

    // Redirect
    http::response::Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", format!("/_/{}/{}", date, head))
        .body(Body::empty()).map_err(warp::reject::custom)
}

fn get_snapshot(
    date: &str,
    db: &mut Connection,
) -> Result<(Option<String>, Option<String>, Option<String>), rusqlite::Error>
{
    let date: Result<(Option<String>, Option<String>, Option<String>), _> =
        if date == "latest"
    {
        db.query_row(
            "
            WITH dates AS (
                SELECT from_date AS date FROM refs
                UNION
                SELECT to_date AS date FROM refs
            )
            SELECT
                (SELECT date FROM dates
                 ORDER BY date DESC LIMIT 1) as current,
                (SELECT date FROM dates
                 ORDER BY date DESC LIMIT 1 OFFSET 1) AS prev,
                NULL AS next;
            ",
            rusqlite::NO_PARAMS,
            |row| (row.get(0), row.get(1), row.get(2)),
        )
    } else {
        db.query_row(
            "
            WITH dates AS (
                SELECT from_date AS date FROM refs
                UNION
                SELECT to_date AS date FROM refs
            )
            SELECT
                (SELECT date FROM dates
                 WHERE date <= ?
                 ORDER BY date DESC LIMIT 1) as current,
                (SELECT date FROM dates
                 WHERE date <= ?
                 ORDER BY date DESC LIMIT 1 OFFSET 1) AS prev,
                (SELECT date FROM dates
                 WHERE date > ?
                 ORDER BY date LIMIT 1) as next;
            ",
            &[&date, &date, &date],
            |row| (row.get(0), row.get(1), row.get(2)),
        )
    };
    match date {
        Ok(triple) => Ok(triple),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok((None, None, None)),
        Err(e) => {
            error!("Error: {}", e);
            Err(e)
        }
    }
}

fn get_branches(
    date: &str,
    db: &mut Connection,
) -> Result<Vec<(String, String)>, rusqlite::Error> {
    let mut stmt = db.prepare(
        "
        SELECT name, sha FROM refs
        WHERE tag=0
            AND from_date <= ?
            AND (to_date IS NULL OR to_date > ?);
        ORDER BY name;
        "
    )?;
    let rows = stmt.query_map(
        &[date, date],
        |row| (
            row.get::<_, String>(0),
            row.get::<_, String>(1),
        ),
    )?;
    let mut branches = Vec::new();
    for branch in rows {
        branches.push(branch?);
    }
    branches.sort();
    Ok(branches)
}

#[derive(Serialize)]
struct Commit {
    sha: String,
    author: String,
    date: String,
    message: String,
}

fn get_commits(
    repository: &Path,
    target: &str,
    number: usize,
) -> Result<Vec<Commit>, String> {
    let output = process::Command::new("git")
        .args(&["log", "--format=short"])
        .arg(format!("{0}~{1}..{0}", target, number))
        .arg("--")
        .current_dir(repository)
        .stdin(process::Stdio::null())
        .output().map_err(|_| "Error running Git")?;
    if !output.status.success() {
        error!("Error running `git log`: {}", output.status);
        return Err(format!("Error running `git log`: {}", output.status));
    }
    let mut commits = Vec::with_capacity(number);
    for line in output.stdout.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(line);
        if line.starts_with("commit ") {
            commits.push(Commit {
                sha: line[7..].into(),
                author: String::new(),
                date: String::new(),
                message: String::new(),
            });
        } else if line.starts_with("Author: ") {
            commits.last_mut().unwrap().author = line.trim().into();
        } else if line.starts_with("Date: ") {
            commits.last_mut().unwrap().date = line.trim().into();
        } else if line.starts_with("    ") {
            commits.last_mut().unwrap().message = line.trim().into();
        }
    }
    Ok(commits)
}

/// Browser view
fn browse(
    date: String,
    refname: String,
    db: Arc<Mutex<Connection>>,
    repository: Arc<PathBuf>,
    templates: Arc<Handlebars>,
) -> Result<impl Reply, warp::reject::Rejection> {
    let date = match percent_encoding::percent_decode(date.as_bytes())
        .decode_utf8()
    {
        Ok(s) => s,
        Err(_) => return Err(warp::reject::not_found()),
    };

    let mut db = db.lock().unwrap();

    // Load snapshot information
    let (current, prev_date, next_date) = match get_snapshot(&date, &mut db)
        .map_err(warp::reject::custom)?
    {
        (Some(current), prev, next) => {
            info!("Resolved date {} -> {}", date, current);
            (current, prev, next)
        }
        (None, _, _) => return Err(warp::reject::not_found()),
    };

    // Load branches
    let mut branches = get_branches(&current, &mut db)
        .map_err(warp::reject::custom)?;
    let current_sha = {
        let idx = branches.binary_search_by(|br| br.0.cmp(&refname))
            .map_err(|_| {
                warn!("Requested branch does not exist");
                warp::reject::not_found()
            })?;
        branches.remove(idx).1
    };

    // Load commits
    let commits = get_commits(
        &repository,
        &current_sha,
        10,
    ).map_err(warp::reject::custom)?;

    // Send response
    templates.render(
        "browse.html",
        &json!({
            "snapshot": {
                "current": current, "prev": prev_date, "next": next_date,
                "req": date,
            },
            "refname": refname,
            "branches": branches,
            "commits": commits,
        }),
    ).map_err(warp::reject::custom).map(warp::reply::html)
}
