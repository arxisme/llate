use rusqlite::{Connection, Result, params};
use regex::Regex;
use std::path::Path;

pub fn init_db(vault_path: &Path) -> Result<Connection> {
    let db_path = vault_path.join("latte.db");
    let conn = Connection::open(&db_path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            path TEXT PRIMARY KEY,
            title TEXT NOT NULL
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS links (
            source TEXT NOT NULL,
            target TEXT NOT NULL,
            PRIMARY KEY(source, target)
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            note TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY(note, tag)
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
            path UNINDEXED,
            title,
            content
        )",
        [],
    )?;
    
    Ok(conn)
}

pub fn update_note_index(conn: &Connection, path: &str, title: &str, content: &str) -> Result<()> {
    // Upsert note
    conn.execute(
        "INSERT INTO notes (path, title) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET title=?2",
        params![path, title],
    )?;
    
    // Clear old links and tags
    conn.execute("DELETE FROM links WHERE source = ?1", params![path])?;
    conn.execute("DELETE FROM tags WHERE note = ?1", params![path])?;
    conn.execute("DELETE FROM notes_fts WHERE path = ?1", params![path])?;
    
    conn.execute(
        "INSERT INTO notes_fts (path, title, content) VALUES (?1, ?2, ?3)",
        params![path, title, content],
    )?;
    
    let link_re = Regex::new(r"\\notelink\{([^}]+)\}").unwrap();
    let tag_re = Regex::new(r"\\notetag\{([^}]+)\}").unwrap();
    
    for cap in link_re.captures_iter(content) {
        if let Some(target) = cap.get(1) {
            let target_str = target.as_str().trim();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO links (source, target) VALUES (?1, ?2)",
                params![path, target_str],
            );
        }
    }
    
    for cap in tag_re.captures_iter(content) {
        if let Some(tag) = cap.get(1) {
            let tag_str = tag.as_str().trim();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO tags (note, tag) VALUES (?1, ?2)",
                params![path, tag_str],
            );
        }
    }
    
    Ok(())
}

pub fn delete_note_index(conn: &Connection, path: &str) -> Result<()> {
    conn.execute("DELETE FROM notes WHERE path = ?1", params![path])?;
    conn.execute("DELETE FROM links WHERE source = ?1", params![path])?;
    conn.execute("DELETE FROM tags WHERE note = ?1", params![path])?;
    conn.execute("DELETE FROM notes_fts WHERE path = ?1", params![path])?;
    Ok(())
}

pub fn get_backlinks(conn: &Connection, target_path: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT source FROM links WHERE target = ?1")?;
    let paths = stmt.query_map(params![target_path], |row| row.get(0))?;
    
    let mut result = Vec::new();
    for p in paths {
        result.push(p?);
    }
    Ok(result)
}

pub fn get_tags_for_note(conn: &Connection, note_path: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT tag FROM tags WHERE note = ?1")?;
    let tags = stmt.query_map(params![note_path], |row| row.get(0))?;
    
    let mut result = Vec::new();
    for t in tags {
        result.push(t?);
    }
    Ok(result)
}

pub fn get_all_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT tag FROM tags ORDER BY tag")?;
    let tags = stmt.query_map([], |row| row.get(0))?;
    
    let mut result = Vec::new();
    for t in tags {
        result.push(t?);
    }
    Ok(result)
}

pub fn search_notes(conn: &Connection, query: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM notes_fts WHERE notes_fts MATCH ?1 ORDER BY rank")?;
    let paths = stmt.query_map(params![query], |row| row.get(0))?;
    
    let mut result = Vec::new();
    for p in paths {
        result.push(p?);
    }
    Ok(result)
}

#[derive(serde::Serialize)]
pub struct GraphData {
    pub nodes: Vec<String>,
    pub links: Vec<(String, String)>,
}

pub fn get_graph_data(conn: &Connection) -> Result<GraphData> {
    let mut stmt_nodes = conn.prepare("SELECT path FROM notes")?;
    let nodes_iter = stmt_nodes.query_map([], |row| row.get(0))?;
    let mut nodes = Vec::new();
    for n in nodes_iter {
        nodes.push(n?);
    }
    
    let mut stmt_links = conn.prepare("SELECT source, target FROM links")?;
    let links_iter = stmt_links.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    let mut links = Vec::new();
    for l in links_iter {
        links.push(l?);
    }
    
    Ok(GraphData { nodes, links })
}
