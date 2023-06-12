use std::fs;

use anyhow::{anyhow, Result};
use rusqlite::Connection;

pub fn extract_cookies(cookies_file_path: &str, hostname: &str) -> Result<String> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push("prnotify-cookies");
    let cookies_filename_temp = temp_path
        .to_str()
        .ok_or_else(|| anyhow!("Could not get temp file path for cookies extraction"))?;
    fs::copy(cookies_file_path, &cookies_filename_temp)?;

    let conn = Connection::open(cookies_filename_temp)?;
    let mut stmt = conn.prepare(&format!(
        "
        SELECT NAME,
               value
        FROM   moz_cookies
        WHERE  host = '{}'
                OR host = '.{}'
        ",
        hostname, hostname
    ))?;
    let cookies = stmt
        .query_map([], |row| {
            Ok(format!(
                "{}={};",
                row.get::<usize, String>(0)?,
                row.get::<usize, String>(1)?
            ))
        })?
        .collect::<Result<Vec<String>, _>>()?
        .join(" ");

    Ok(cookies)
}
