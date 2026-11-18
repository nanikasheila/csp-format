//! Brush styles for new vector lines whose width changes along the line.
//!
//! A vector object's points keep a width factor (point value 0, the pen
//! pressure; [`crate::vector_build::NewVectorLine`]), but CSP sizes a point
//! by it only when the object's brush style follows the pressure
//! (`BrushStyle.SizeEffector` flag 0x10): `size × (min + (1 − min) ×
//! curve(pressure))`. A style without that flag (`SizeEffector` 0x01, as the
//! vector layers of many templates have) draws every point at the full size,
//! and a pen's own curve would bend factors already worked out once more.
//!
//! [`ClipFile::pressure_brush_style`] gives a copy of a style whose size
//! follows the pressure one to one (minimum 0, a straight graph), so each
//! point is drawn as wide as its factor says. CSP draws such a style this
//! way (round `b` of `csp-lab`'s `vec_test`, `docs/validation/V6-made-files.md`
//! section 7). The new rows are chained where CSP keeps them
//! (`BrushStyleManager.FirstBrushStyle` / `FirstGraphData`, then `NextIndex`).

use std::collections::{HashMap, HashSet};

use rusqlite::types::Value;

use crate::clip::ClipFile;
use crate::copy::insert_row;
use crate::database::{Row, has_column, has_table, row, select};
use crate::error::Result;
use crate::stroke::{Curve, Effector};

/// `SizeEffector` flags of a size that follows the pressure (0x10) and
/// nothing else (0x01 is set in every effector CSP writes).
const PRESSURE_FLAGS: u32 = 0x11;
/// A straight graph, as CSP's own: three control points.
const LINEAR_GRAPH: [(f64, f64); 3] = [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)];
/// The table as CSP creates it.
const GRAPH_TABLE_SQL: &str = "CREATE TABLE BrushEffectorGraphData(_PW_ID INTEGER PRIMARY KEY AUTOINCREMENT, \
     MainId INTEGER DEFAULT NULL, CanvasId INTEGER DEFAULT NULL, NextIndex INTEGER DEFAULT NULL, \
     ControlNumber INTEGER DEFAULT NULL, ControlDataSize INTEGER DEFAULT NULL, ControlPoints BLOB DEFAULT NULL)";
const GRAPH_TABLE: &str = "BrushEffectorGraphData";
/// Columns a copy of a style may differ in from its original.
const OWN_COLUMNS: [&str; 4] = ["_PW_ID", "MainId", "NextIndex", "SizeEffector"];
/// Longest `NextIndex` chain followed (a loop in a broken file stops here).
const CHAIN_LIMIT: usize = 100_000;

/// The `SizeEffector` blob following the pressure one to one through graph `graph`.
fn pressure_effector(graph: i64) -> Vec<u8> {
    [PRESSURE_FLAGS, 0f32.to_bits(), graph as u32].iter().flat_map(|word| word.to_be_bytes()).collect()
}

fn graph_points() -> Vec<u8> {
    LINEAR_GRAPH.iter().flat_map(|(x, y)| x.to_be_bytes().into_iter().chain(y.to_be_bytes())).collect()
}

/// Whether a style's `SizeEffector` sizes a point by its pressure as it is.
fn follows_pressure_as_is(effector: &[u8], curves: &HashMap<i64, Curve>) -> bool {
    let flags = effector.get(..4).map_or(0, |word| u32::from_be_bytes(word.try_into().expect("4")));
    if flags != PRESSURE_FLAGS {
        return false;
    }
    let parsed = Effector::parse(effector, curves);
    let Some((minimum, curve)) = &parsed.pressure else { return false };
    let graph = effector.get(8..12).map_or(0, |word| u32::from_be_bytes(word.try_into().expect("4")));
    *minimum == 0.0
        && curves.contains_key(&(graph as i64))
        && (0..=20).all(|step| {
            let x = f64::from(step) / 20.0;
            (curve.at(x) - x).abs() < 1e-6
        })
}

impl ClipFile {
    /// A brush style drawn like `base` (a `BrushStyle.MainId` of this file)
    /// whose size follows each point's width factor one to one, for new
    /// vector lines whose width changes along them. `base` itself when it
    /// already does, or when the file keeps no brush styles to add to; an
    /// earlier copy of `base` when there is one.
    pub fn pressure_brush_style(&mut self, base: u32) -> Result<u32> {
        let database = self.database();
        if !has_table(database, "BrushStyle")? || !has_table(database, "BrushStyleManager")? {
            return Ok(base);
        }
        let Some(original) = row(database, "BrushStyle", i64::from(base))? else { return Ok(base) };
        let Some(manager) = brush_style_manager(self)? else { return Ok(base) };
        let curves = graphs(self)?;
        if follows_pressure_as_is(original.blob("SizeEffector").unwrap_or_default(), &curves) {
            return Ok(base);
        }
        for style in select(self.database(), "SELECT * FROM BrushStyle", [])? {
            if follows_pressure_as_is(style.blob("SizeEffector").unwrap_or_default(), &curves)
                && same_apart_from_size(&style, &original)
            {
                return Ok(style.int("MainId") as u32);
            }
        }

        if !has_table(self.database(), GRAPH_TABLE)? {
            self.database().execute(GRAPH_TABLE_SQL, [])?;
            crate::create::register_table(self.database(), GRAPH_TABLE)?;
        }
        let graph = self.next_main_id(GRAPH_TABLE)?;
        let canvas = original.get("CanvasId").cloned().unwrap_or(Value::Integer(1));
        let points = graph_points();
        let graph_row = Row::new(
            ["MainId", "CanvasId", "NextIndex", "ControlNumber", "ControlDataSize", "ControlPoints"]
                .map(str::to_owned)
                .to_vec(),
            vec![
                Value::Integer(graph),
                canvas,
                Value::Integer(0),
                Value::Integer(LINEAR_GRAPH.len() as i64),
                Value::Integer(16),
                Value::Blob(points),
            ],
        );
        insert_row(self.database(), GRAPH_TABLE, &graph_row)?;
        append_to_chain(self, manager, "FirstGraphData", GRAPH_TABLE, graph)?;

        let style = self.next_main_id("BrushStyle")?;
        let mut copy = original;
        copy.set("MainId", Value::Integer(style));
        copy.set("NextIndex", Value::Integer(0));
        copy.set("SizeEffector", Value::Blob(pressure_effector(graph)));
        let kept: Vec<String> = copy.columns().iter().filter(|column| *column != "_PW_ID").cloned().collect();
        let values = kept.iter().map(|column| copy.get(column).cloned().unwrap_or(Value::Null)).collect();
        insert_row(self.database(), "BrushStyle", &Row::new(kept, values))?;
        append_to_chain(self, manager, "FirstBrushStyle", "BrushStyle", style)?;
        Ok(style as u32)
    }
}

/// The `BrushStyleManager` the canvas names (or the only one).
fn brush_style_manager(file: &ClipFile) -> Result<Option<i64>> {
    let named = if has_column(file.database(), "Canvas", "BrushStyleManager")? {
        match file.canvas_value("BrushStyleManager")? {
            Some(Value::Integer(id)) if id > 0 => Some(id),
            _ => None,
        }
    } else {
        None
    };
    let rows = select(file.database(), "SELECT MainId FROM BrushStyleManager", [])?;
    Ok(match named {
        Some(id) if rows.iter().any(|row| row.int("MainId") == id) => Some(id),
        _ if rows.len() == 1 => Some(rows[0].int("MainId")),
        _ => None,
    })
}

fn graphs(file: &ClipFile) -> Result<HashMap<i64, Curve>> {
    let mut curves = HashMap::new();
    if has_table(file.database(), GRAPH_TABLE)? {
        for row in select(file.database(), "SELECT * FROM BrushEffectorGraphData", [])? {
            let count = row.opt_int("ControlNumber").and_then(|n| usize::try_from(n).ok());
            curves.insert(row.int("MainId"), Curve::parse(row.blob("ControlPoints").unwrap_or_default(), count));
        }
    }
    Ok(curves)
}

fn same_apart_from_size(style: &Row, original: &Row) -> bool {
    style.int("MainId") != original.int("MainId")
        && original
            .iter()
            .filter(|(column, _)| !OWN_COLUMNS.contains(column))
            .all(|(column, value)| style.get(column) == Some(value))
}

/// Put row `id` of `table` at the end of the list that starts at the
/// manager's `first` column and goes on through `NextIndex`.
fn append_to_chain(file: &ClipFile, manager: i64, first: &str, table: &str, id: i64) -> Result<()> {
    let database = file.database();
    if !has_column(database, "BrushStyleManager", first)? {
        return Ok(());
    }
    let head: Option<i64> =
        database.query_row(&format!("SELECT {first} FROM BrushStyleManager WHERE MainId = ?1"), [manager], |row| {
            row.get(0)
        })?;
    let head = head.unwrap_or(0);
    if head <= 0 || head == id {
        database.execute(&format!("UPDATE BrushStyleManager SET {first} = ?1 WHERE MainId = ?2"), [id, manager])?;
        return Ok(());
    }
    let mut seen = HashSet::new();
    let mut current = head;
    for _ in 0..CHAIN_LIMIT {
        if !seen.insert(current) {
            break;
        }
        let next: Option<i64> = database
            .query_row(&format!("SELECT NextIndex FROM {table} WHERE MainId = ?1"), [current], |row| row.get(0))
            .ok()
            .flatten();
        match next {
            Some(next) if next > 0 && next != id && row(database, table, next)?.is_some() => current = next,
            _ => break,
        }
    }
    database.execute(&format!("UPDATE {table} SET NextIndex = ?1 WHERE MainId = ?2"), [id, current])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::NewCanvas;
    use crate::stroke::brushes;

    /// A new file with the brush tables of a template's vector layer: one
    /// style whose size does not follow the pressure, as the built-in
    /// templates have, and one more after it.
    fn file_with_styles() -> ClipFile {
        let mut file = ClipFile::create(&NewCanvas::default()).unwrap();
        let database = file.database();
        database
            .execute_batch(
                "CREATE TABLE BrushStyleManager(_PW_ID INTEGER PRIMARY KEY AUTOINCREMENT, MainId INTEGER, CanvasId INTEGER, \
                   FirstGraphData INTEGER, FirstBrushStyle INTEGER);
                 CREATE TABLE BrushStyle(_PW_ID INTEGER PRIMARY KEY AUTOINCREMENT, MainId INTEGER, CanvasId INTEGER, \
                   NextIndex INTEGER, SizeEffector BLOB, AntiAlias INTEGER, ThicknessBase REAL);
                 INSERT INTO BrushStyleManager VALUES (1, 8, 1, 0, 33);
                 INSERT INTO BrushStyle VALUES (1, 33, 1, 34, x'00000001', 1, 1.0);
                 INSERT INTO BrushStyle VALUES (2, 34, 1, 0, x'00000001', 3, 1.0);
                 INSERT INTO ElemScheme (TableName, ElemType, MaxIndex) VALUES ('BrushStyle', 0, 34);",
            )
            .unwrap();
        file.set_canvas_value("BrushStyleManager", Value::Integer(8)).unwrap();
        file
    }

    fn reopened(file: &ClipFile) -> ClipFile {
        ClipFile::from_bytes(file.to_bytes().unwrap()).unwrap()
    }

    #[test]
    fn a_copy_follows_the_point_factors() {
        let mut file = file_with_styles();
        let style = file.pressure_brush_style(33).unwrap();
        assert_ne!(style, 33);
        let file = reopened(&file);
        let brushes = brushes(&file).unwrap();
        for factor in [0.05, 0.3, 0.72, 1.0] {
            assert!((brushes[&(style as i64)].size.factor(factor, 0.0) - factor).abs() < 1e-6, "{factor}");
            assert_eq!(brushes[&33].size.factor(factor, 0.0), 1.0, "the original stays as it was");
        }
        // Everything else is the original's.
        let copy = row(file.database(), "BrushStyle", i64::from(style)).unwrap().unwrap();
        assert_eq!(copy.int("AntiAlias"), 1);
        assert_eq!(copy.int("NextIndex"), 0);
        // Chained after the last style, its graph the manager's first.
        assert_eq!(row(file.database(), "BrushStyle", 34).unwrap().unwrap().int("NextIndex"), i64::from(style));
        let manager = row(file.database(), "BrushStyleManager", 8).unwrap().unwrap();
        let graph = manager.int("FirstGraphData");
        let graph_row = row(file.database(), GRAPH_TABLE, graph).unwrap().unwrap();
        assert_eq!(graph_row.int("ControlNumber"), 3);
        assert_eq!(graph_row.int("NextIndex"), 0);
        // The new table is registered as CSP registers it.
        let labels =
            select(file.database(), "SELECT LabelName FROM ParamScheme WHERE TableName = ?1", [GRAPH_TABLE]).unwrap();
        assert_eq!(labels.len(), 5);
        let report = crate::verify::check_clip(file.to_bytes().unwrap(), Default::default());
        assert!(report.ok(), "{:?}", report.errors);
    }

    #[test]
    fn the_copy_is_made_once() {
        let mut file = file_with_styles();
        let first = file.pressure_brush_style(33).unwrap();
        assert_eq!(file.pressure_brush_style(33).unwrap(), first);
        assert_eq!(file.pressure_brush_style(first).unwrap(), first, "a style that follows as is is kept");
        let other = file.pressure_brush_style(34).unwrap();
        assert_ne!(other, first, "34 draws with another anti-aliasing");
        let count: i64 = file.database().query_row("SELECT COUNT(*) FROM BrushStyle", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 4);
        // Each copy has its graph, the second chained after the first.
        let graphs =
            select(file.database(), "SELECT MainId, NextIndex FROM BrushEffectorGraphData ORDER BY MainId", [])
                .unwrap();
        assert_eq!(graphs.len(), 2);
        assert_eq!(graphs[0].int("NextIndex"), graphs[1].int("MainId"));
        assert_eq!(graphs[1].int("NextIndex"), 0);
    }

    #[test]
    fn files_without_brush_styles_keep_the_id() {
        let mut file = ClipFile::create(&NewCanvas::default()).unwrap();
        assert_eq!(file.pressure_brush_style(5).unwrap(), 5);
        let mut file = file_with_styles();
        assert_eq!(file.pressure_brush_style(99).unwrap(), 99, "no such style");
    }
}
