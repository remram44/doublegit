use handlebars::Handlebars;
use http::StatusCode;
use hyper::Body;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use warp::{self, Filter};
use warp::path;
use warp::reply::Response;

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
        // Browse view, shows a branch in a snapshot
        .or(path!("_" / String / String)
            .and(db).and(templates).and_then(browse));

    println!("\n    Starting server on {}:{}\n", host, port);
    warp::serve(routes).run((host, port));

    Ok(())
}

/// Redirects to main branch in latest snapshot
fn index(
    db: Arc<Mutex<Connection>>
) -> Result<Response, warp::reject::Rejection> {
    // First we have to find a suitable branch
    let head = (|| -> Result<String, rusqlite::Error> {
        let db = db.lock().unwrap();
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

fn get_snapshot(
    time: &str,
    db: &mut Connection,
) -> Result<Option<String>, rusqlite::Error> {
    let date: Result<Option<String>, _> = if time == "latest" {
        db.query_row(
            "
            SELECT MAX(date)
            FROM (
                SELECT MAX(from_date) AS date FROM refs
                UNION ALL
                SELECT MAX(to_date) AS date FROM refs
            );
            ",
            rusqlite::NO_PARAMS,
            |row| row.get(0),
        )
    } else {
        db.query_row(
            "
            SELECT MAX(date)
            FROM (
                SELECT from_date AS date FROM refs
                WHERE from_date <= ?
                UNION ALL
                SELECT MAX(to_date) AS date FROM refs
                WHERE to_date <= ?
            );
            ",
            &[&time, &time],
            |row| row.get(0),
        )
    };
    match date {
        Ok(maybe_date) => Ok(maybe_date),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => {
            error!("Error: {}", e);
            Err(e)
        }
    }
}

/// Browser view
fn browse(
    time: String,
    refname: String,
    db: Arc<Mutex<Connection>>,
    templates: Arc<Handlebars>,
) -> Result<String, warp::reject::Rejection> {
    let time = match percent_encoding::percent_decode(time.as_bytes())
        .decode_utf8()
    {
        Ok(s) => s,
        Err(_) => return Err(warp::reject::not_found()),
    };

    let mut db = db.lock().unwrap();

    // Load snapshot information
    let date = match get_snapshot(&time, &mut db)
        .map_err(warp::reject::custom)?
    {
        Some(date) => date,
        None => return Err(warp::reject::not_found()),
    };

    // Send response
    info!("Resolved date {} -> {}", time, date);
    templates.render(
        "browse.html",
        &json!({"snapshot": date, "refname": refname}),
    ).map_err(warp::reject::custom)
}
