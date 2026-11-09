//! The database written to (`data/db.py`: connect, cleanup), a ready database put in place at the next start
//! (`data/ready.py`) and back to zero (`core/reset.py`).

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::tables;
use crate::{bookmarks, cache, config, paths};

pub const LOCK_WAIT: u64 = 15;
pub const INCOMING: &str = ".incoming";

static STARTED: AtomicBool = AtomicBool::new(false);

/// A staged ready database takes the place of the current one at the first connection of a start (`db.connect`).
pub fn first_connect() {
    if !STARTED.swap(true, Ordering::SeqCst) {
        let _ = apply_pending();
    }
}

/// Open the database to write to it, creating or upgrading it as the Python isnady does (`db.connect`).
pub fn connect_rw(path: Option<&Path>) -> Result<Connection, String> {
    if path.is_none() {
        first_connect();
    }
    let path = path.map(PathBuf::from).unwrap_or_else(paths::db_path);
    let t = tables();
    if path.exists() {
        let probe = Connection::open(&path).map_err(|e| e.to_string())?;
        let version: i64 = probe
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        drop(probe);
        if version > t.schema_version {
            return Err(format!(
                "Database schema {version} is newer than this isnady supports ({}). Update isnady.",
                t.schema_version
            ));
        }
        if 0 < version && version < t.oldest_upgradable {
            return Err(format!(
                "Database schema {version} is outdated; open it once with the Python isnady, which sets it aside."
            ));
        }
        if t.oldest_upgradable <= version && version < t.schema_version {
            let mut up = Connection::open(&path).map_err(|e| e.to_string())?;
            let tx = up.transaction().map_err(|e| e.to_string())?;
            for v in version..t.schema_version {
                if let Some((_, stmts)) = t.migrations.iter().find(|(n, _)| *n == v) {
                    for s in stmts {
                        tx.execute_batch(s).map_err(|e| e.to_string())?;
                    }
                }
            }
            tx.execute_batch(&format!("PRAGMA user_version = {}", t.schema_version))
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
    } else if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(LOCK_WAIT))
        .map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .map_err(|e| e.to_string())?;
    let _: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    conn.execute_batch(&t.schema).map_err(|e| e.to_string())?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version != t.schema_version {
        conn.execute_batch(&format!("PRAGMA user_version = {}", t.schema_version))
            .map_err(|e| e.to_string())?;
    }
    Ok(conn)
}

/// The hadith, sections and books nothing points to any more (`db.cleanup`).
pub fn cleanup(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(&tables().cleanup)
}

/// Remove a source and everything imported from it (`imports.remove_source`).
pub fn remove_source(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    let removed = conn.execute("DELETE FROM sources WHERE key = ?", [key])?;
    cleanup(conn)?;
    Ok(removed > 0)
}

// ------------------------------------------------------------------ the search index, empty again
fn fts_available(conn: &Connection) -> bool {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS temp._isnady_probe USING fts5(x, tokenize='trigram');
         DROP TABLE temp._isnady_probe;",
    )
    .is_ok()
}

pub(crate) fn create_index_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS text_norm (
             text_id INTEGER PRIMARY KEY REFERENCES texts(id) ON DELETE CASCADE,
             norm    TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS words (word TEXT PRIMARY KEY, n INTEGER NOT NULL) WITHOUT ROWID;",
    )?;
    if fts_available(conn) {
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS texts_fts USING fts5(
                 norm, content='text_norm', content_rowid='text_id', tokenize='trigram'
             );
             CREATE TRIGGER IF NOT EXISTS text_norm_ai AFTER INSERT ON text_norm BEGIN
                 INSERT INTO texts_fts(rowid, norm) VALUES (new.text_id, new.norm);
             END;
             CREATE TRIGGER IF NOT EXISTS text_norm_ad AFTER DELETE ON text_norm BEGIN
                 INSERT INTO texts_fts(texts_fts, rowid, norm) VALUES ('delete', old.text_id, old.norm);
             END;",
        )?;
    }
    Ok(())
}

// ------------------------------------------------------------------ back to zero (RST1)
/// Remove every imported source and what came with it; how many sources there were (`reset.books`).
pub fn reset_books(conn: &Connection) -> rusqlite::Result<i64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM sources", [])?;
    tx.execute("DELETE FROM persons", [])?;
    tx.commit()?;
    cleanup(conn)?;
    crate::search::rebuild_index(conn, &|_| {})?; // the search index and its word list, now empty
    let _ = conn.execute_batch("VACUUM");
    cache::clear();
    crate::log::warning("reset", &format!("⟲ every book removed ({n} sources)"));
    Ok(n)
}

pub struct Everything {
    pub books: i64,
    pub settings: usize,
    pub bookmarks: bool,
}

/// Books, settings, bookmarks, downloaded repositories: all of the profile in use (`reset.everything`).
pub fn reset_everything(conn: &Connection) -> rusqlite::Result<Everything> {
    let books = reset_books(conn)?;
    let settings = config::reset_settings().unwrap_or(0);
    crate::log::warning(
        "reset",
        &format!("⟲ every setting reset ({settings} stored)"),
    );
    let bookmarks = std::fs::remove_file(bookmarks::path()).is_ok();
    let repos = config::profile_folder().join("repos");
    if repos.exists() {
        let _ = std::fs::remove_dir_all(&repos);
    }
    super::repos::forget();
    Ok(Everything {
        books,
        settings,
        bookmarks,
    })
}

// ------------------------------------------------------------------ a ready database (data/ready.py)
pub fn incoming_path() -> PathBuf {
    let db = paths::db_path();
    let name = format!(
        "{}{INCOMING}",
        db.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("isnady.db")
    );
    db.with_file_name(name)
}

pub fn pending() -> bool {
    incoming_path().exists()
}

pub struct ReadyInfo {
    pub schema: i64,
    pub books: i64,
    pub hadith: i64,
}

fn check(path: &Path) -> Result<ReadyInfo, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let read = || -> rusqlite::Result<ReadyInfo> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(ReadyInfo {
            schema: conn.query_row("PRAGMA user_version", [], |r| r.get(0))?,
            books: conn.query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))?,
            hadith: conn.query_row("SELECT COUNT(*) FROM hadiths", [], |r| r.get(0))?,
        })
    };
    let info = read().map_err(|e| format!("Not an isnady database: {name} ({e})"))?;
    if info.schema > tables().schema_version {
        return Err(format!(
            "The database is newer (schema {}) than this isnady reads ({}). Update isnady first.",
            info.schema,
            tables().schema_version
        ));
    }
    Ok(info)
}

fn sha256_hex(path: &Path) -> std::io::Result<String> {
    // the same digest as hashlib.sha256 (a small, plain implementation: one file, once)
    let data = std::fs::read(path)?;
    Ok(sha256::hex(&data))
}

/// Check a database file (isnady.db, or a zip holding it) and stage it for the next start (`ready.stage`).
pub fn stage(file: &str) -> Result<ReadyInfo, String> {
    let src = paths::expand_user(file);
    if !src.is_file() {
        return Err(format!("File not found: {}", src.display()));
    }
    let target = incoming_path();
    if let Some(dir) = target.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = target.with_file_name(format!(
        "{}.part",
        target.file_name().unwrap().to_string_lossy()
    ));
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut head = [0u8; 4];
    let is_zip = std::fs::File::open(&src)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && head == *b"PK\x03\x04";
    if is_zip {
        let sidecar = src.with_file_name("database.json");
        if sidecar.exists() {
            let expected = std::fs::read_to_string(&sidecar)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| v.get("sha256").and_then(|x| x.as_str()).map(str::to_string))
                .filter(|x| !x.is_empty());
            if let Some(expected) = expected {
                if sha256_hex(&src).map_err(|e| e.to_string())? != expected {
                    return Err(format!(
                        "{name} does not match the sha256 in database.json beside it: the download is incomplete \
                         or the file was changed."
                    ));
                }
            }
        }
        let f = std::fs::File::open(&src).map_err(|e| e.to_string())?;
        let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        let inner = (0..z.len())
            .filter_map(|i| z.by_index(i).ok().map(|e| e.name().to_string()))
            .find(|n| n.ends_with("isnady.db"))
            .ok_or_else(|| format!("{name} holds no isnady.db."))?;
        let mut r = z.by_name(&inner).map_err(|e| e.to_string())?;
        let mut w = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        std::io::copy(&mut r, &mut w).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(&src, &tmp).map_err(|e| e.to_string())?;
    }
    match check(&tmp) {
        Ok(info) => {
            std::fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
            Ok(info)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Put a staged database in place; the database it replaces is kept (`ready.apply_pending`). The backup's path.
pub fn apply_pending() -> std::io::Result<Option<PathBuf>> {
    let incoming = incoming_path();
    if !incoming.exists() {
        return Ok(None);
    }
    let current = paths::db_path();
    let mut backup = None;
    if current.exists() {
        let b = current.with_file_name(format!(
            "isnady.{}.bak.db",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
        std::fs::rename(&current, &b)?;
        for extra in ["-wal", "-shm"] {
            let side = current.with_file_name(format!("isnady.db{extra}"));
            if side.exists() {
                std::fs::rename(
                    &side,
                    b.with_file_name(format!(
                        "{}{extra}",
                        b.file_name().unwrap().to_string_lossy()
                    )),
                )?;
            }
        }
        backup = Some(b);
    }
    std::fs::rename(&incoming, &current)?;
    Ok(backup)
}

/// Whether a source with this key is imported.
pub fn has_source(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM sources WHERE key = ?", [key], |r| {
            r.get::<_, i64>(0)
        })
        .optional()?
        .is_some())
}

mod sha256 {
    //! SHA-256 (FIPS 180-4), for the database.json check of a ready database.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    pub fn hex(data: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut msg = data.to_vec();
        let bits = (data.len() as u64).wrapping_mul(8);
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bits.to_be_bytes());
        for chunk in msg.chunks(64) {
            let mut w = [0u32; 64];
            for (i, word) in chunk.chunks(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let mut v = h;
            for i in 0..64 {
                let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
                let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
                let t1 = v[7]
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
                let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
                let t2 = s0.wrapping_add(maj);
                v = [
                    t1.wrapping_add(t2),
                    v[0],
                    v[1],
                    v[2],
                    v[3].wrapping_add(t1),
                    v[4],
                    v[5],
                    v[6],
                ];
            }
            for i in 0..8 {
                h[i] = h[i].wrapping_add(v[i]);
            }
        }
        h.iter().map(|x| format!("{x:08x}")).collect()
    }

    #[cfg(test)]
    #[test]
    fn as_hashlib() {
        assert_eq!(
            hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
