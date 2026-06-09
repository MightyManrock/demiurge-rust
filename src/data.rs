use crate::bio;

use rusqlite::Connection;
use serde_json;

fn write_species(conn: &Connection, species_list: Vec<bio::Species>) {

    conn.execute_batch("
      CREATE TABLE IF NOT EXISTS species (
          id          INTEGER PRIMARY KEY,
          name        TEXT,
          kind        TEXT NOT NULL,
          basis       TEXT NOT NULL,
          solvent     TEXT NOT NULL,
          atmo_aff    TEXT NOT NULL,
          food_tag    TEXT NOT NULL,
          temp_range  TEXT,
          press_range TEXT,
          grav_range  TEXT
        );
    ").unwrap();

    for species in species_list {
        conn.execute(
            "INSERT INTO species (name, kind, basis, solvent, atmo_aff, food_tag, temp_range, press_range, grav_range)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                species.name,
                serde_json::to_string(&species.kind).unwrap(),
                serde_json::to_string(&species.basis).unwrap(),
                serde_json::to_string(&species.solvent).unwrap(),
                serde_json::to_string(&species.atmo_aff).unwrap(),
                serde_json::to_string(&species.food_tag).unwrap(),
                serde_json::to_string(&species.temp_range).unwrap(),
                serde_json::to_string(&species.press_range).unwrap(),
                serde_json::to_string(&species.grav_range).unwrap(),
            ],
        ).unwrap();
    }
}

pub(crate) fn write_db(path: &str, species_list: Vec<bio::Species>) {

    let conn = Connection::open(path).unwrap();

    if !species_list.is_empty() {
        write_species(&conn, species_list);
    }

}

pub(crate) fn read_db(path: &str) -> Vec<bio::Species> {
    
    let conn = Connection::open(path).unwrap();

    let mut stmt = conn.prepare(
        "SELECT name, kind, basis, solvent, atmo_aff, food_tag, temp_range, press_range, grav_range FROM species"
    ).unwrap();

    let species_list: Vec<bio::Species> = stmt.query_map([], |row| {
        Ok(bio::Species {
            name: row.get(0)?,
            kind: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
            basis: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
            solvent: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
            atmo_aff: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
            food_tag: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
            temp_range: serde_json::from_str(&row.get::<_, String>(6)?).unwrap(),
            press_range: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
            grav_range: serde_json::from_str(&row.get::<_, String>(8)?).unwrap(),
        })
    }).unwrap()
        .map(|s| s.unwrap())
        .collect();

    return species_list;
}
