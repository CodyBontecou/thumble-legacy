//! Three-way raw owned edits, preserving opaque values and exact numeric tokens.
use crate::drafts::DraftError;
use serde_json::{value::RawValue, Value};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};
use thumble_core::OwnedConfigurationDocument;
use thumble_protocol::{edit_json_values, JsonValueEdit};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;
const MAXIMUM: usize = 8 * 1024 * 1024;
pub(super) fn merge(
    base: &OwnedConfigurationDocument,
    working: &OwnedConfigurationDocument,
    current: &OwnedConfigurationDocument,
) -> Result<OwnedConfigurationDocument, DraftError> {
    let raw = |data| serde_json::from_slice::<&RawValue>(data).map_err(|_| DraftError::Malformed);
    let mut merger = Merger {
        remaining: MAXIMUM * 4,
    };
    let output = merger
        .value(
            Some(raw(base.original_bytes())?),
            Some(raw(working.original_bytes())?),
            Some(raw(current.original_bytes())?),
            "",
            0,
        )?
        .ok_or(DraftError::Malformed)?;
    OwnedConfigurationDocument::decode(output.as_bytes())
        .map_err(|_| DraftError::MergeConflict(vec!["/".into()]))
}
struct Field<'a> {
    key: String,
    value: &'a RawValue,
}
fn fields(value: &RawValue) -> Result<BTreeMap<String, Field<'_>>, DraftError> {
    let values: BTreeMap<String, &RawValue> =
        serde_json::from_str(value.get()).map_err(|_| DraftError::Malformed)?;
    Ok(values
        .into_iter()
        .map(|(key, value)| (key.nfc().collect(), Field { key, value }))
        .collect())
}
fn rows(value: &RawValue) -> Result<Vec<&RawValue>, DraftError> {
    serde_json::from_str(value.get()).map_err(|_| DraftError::Malformed)
}
fn equal(a: Option<&RawValue>, b: Option<&RawValue>) -> bool {
    a.map(RawValue::get) == b.map(RawValue::get)
}
fn conflict(path: &str) -> DraftError {
    DraftError::MergeConflict(vec![if path.is_empty() {
        "/".into()
    } else {
        path.into()
    }])
}
fn path(parent: &str, key: &str) -> Result<String, DraftError> {
    if key.len() > 512 {
        return Err(conflict(parent));
    }
    let result = format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"));
    if result.len() > 512 {
        return Err(conflict(parent));
    }
    Ok(result)
}
fn identity(path: &str, value: &RawValue) -> Option<String> {
    let value: Value = serde_json::from_str(value.get()).ok()?;
    if path.ends_with("/appearanceBank/variants") {
        return Some(format!(
            "{}/{}",
            value["colorScheme"].as_str()?,
            value["actionSet"].as_str()?
        ));
    }
    if path.ends_with("/designMetadata/layerOrder") || path.ends_with("/children") {
        if let Some(key) = value.as_str() {
            return Some(key.nfc().collect());
        }
        if let Some(kind) = value["kind"].as_str() {
            let key = match kind {
                "builtin" => "button",
                "custom" => "id",
                "system" => "system",
                "controlBarItem" => "item",
                _ => return None,
            };
            return Some(format!("{kind}:{}", value[key].as_str()?.to_lowercase()));
        }
    }
    let id = value.get("id")?.as_str()?;
    Some(Uuid::parse_str(id).map_or_else(|_| id.nfc().collect(), |id| id.to_string()))
}
fn identified<'a>(
    path: &str,
    values: &[&'a RawValue],
) -> Option<(Vec<String>, BTreeMap<String, &'a RawValue>)> {
    let mut ids = Vec::new();
    let mut map = BTreeMap::new();
    for value in values {
        let id = identity(path, value)?;
        if map.insert(id.clone(), *value).is_some() {
            return None;
        }
        ids.push(id);
    }
    Some((ids, map))
}
fn pair_map(path: &str) -> bool {
    let pieces: Vec<_> = path.split('/').collect();
    pieces.len() == 5
        && pieces[1] == "profiles"
        && pieces[3] == "customization"
        && [
            "buttonCustomizations",
            "labelOverrides",
            "controlBarItemCustomizations",
        ]
        .contains(&pieces[4])
}
type PairedValues<'a> = (Vec<String>, BTreeMap<String, (&'a RawValue, &'a RawValue)>);
fn paired<'a>(values: &[&'a RawValue]) -> Option<PairedValues<'a>> {
    if values.len() % 2 != 0 {
        return None;
    }
    let mut ids = vec![];
    let mut map = BTreeMap::new();
    for pair in values.chunks_exact(2) {
        let text: String = serde_json::from_str(pair[0].get()).ok()?;
        let id = Uuid::parse_str(&text).map_or_else(|_| text.nfc().collect(), |id| id.to_string());
        if map.insert(id.clone(), (pair[0], pair[1])).is_some() {
            return None;
        }
        ids.push(id);
    }
    Some((ids, map))
}
struct Merger {
    remaining: usize,
}
impl Merger {
    fn value<'a>(
        &mut self,
        b: Option<&'a RawValue>,
        w: Option<&'a RawValue>,
        c: Option<&'a RawValue>,
        at: &str,
        depth: usize,
    ) -> Result<Option<Cow<'a, str>>, DraftError> {
        if depth > 128 || self.remaining == 0 {
            return Err(conflict(at));
        }
        self.remaining -= 1;
        if equal(b, w) || equal(w, c) {
            return Ok(c.map(|v| Cow::Borrowed(v.get())));
        }
        if equal(b, c) {
            return Ok(w.map(|v| Cow::Borrowed(v.get())));
        }
        let (Some(b), Some(w), Some(c)) = (b, w, c) else {
            return Err(conflict(at));
        };
        let pieces: Vec<_> = at.split('/').collect();
        if pieces.len() == 4 && pieces[1] == "profiles" && pieces[3] == "updatedAt" {
            let integer = |raw: &RawValue| {
                let value: Value = serde_json::from_str(raw.get()).ok()?;
                value.as_i64().or_else(|| {
                    value
                        .as_f64()
                        .filter(|n| {
                            n.is_finite()
                                && *n >= 0.
                                && *n < 9_223_372_036_854_775_808.
                                && n.fract() == 0.
                        })
                        .map(|n| n as i64)
                })
            };
            let (Some(wi), Some(ci)) = (integer(w), integer(c)) else {
                return Err(conflict(at));
            };
            return Ok(Some(Cow::Borrowed(if wi > ci { w.get() } else { c.get() })));
        }
        match (
            b.get().as_bytes()[0],
            w.get().as_bytes()[0],
            c.get().as_bytes()[0],
        ) {
            (b'{', b'{', b'{') => self.object(b, w, c, at, depth + 1).map(Some),
            (b'[', b'[', b'[') => self.array(b, w, c, at, depth + 1).map(Some),
            _ => Err(conflict(at)),
        }
    }
    fn object<'a>(
        &mut self,
        b: &'a RawValue,
        w: &'a RawValue,
        c: &'a RawValue,
        at: &str,
        depth: usize,
    ) -> Result<Cow<'a, str>, DraftError> {
        let (base, work, current) = (fields(b)?, fields(w)?, fields(c)?);
        let keys: BTreeSet<_> = base
            .keys()
            .chain(work.keys())
            .chain(current.keys())
            .collect();
        let mut changes = vec![];
        for key in keys {
            let (b, w, c) = (base.get(key), work.get(key), current.get(key));
            if equal(b.map(|f| f.value), w.map(|f| f.value))
                || equal(w.map(|f| f.value), c.map(|f| f.value))
            {
                continue;
            }
            let merged = self.value(
                b.map(|f| f.value),
                w.map(|f| f.value),
                c.map(|f| f.value),
                &path(at, key)?,
                depth,
            )?;
            if merged.as_deref() == c.map(|f| f.value.get()) {
                continue;
            }
            let field = c.or(w).or(b).ok_or(DraftError::Malformed)?;
            changes.push((path("", &field.key)?, merged.map(Cow::into_owned)));
        }
        let edits: Vec<_> = changes
            .iter()
            .map(|(pointer, value)| JsonValueEdit {
                pointer,
                value: value.as_ref().map(|v| v.as_bytes()),
            })
            .collect();
        let bytes =
            edit_json_values(c.get().as_bytes(), &edits, MAXIMUM).map_err(|_| conflict(at))?;
        String::from_utf8(bytes)
            .map(Cow::Owned)
            .map_err(|_| DraftError::Malformed)
    }
    fn array<'a>(
        &mut self,
        b: &'a RawValue,
        w: &'a RawValue,
        c: &'a RawValue,
        at: &str,
        depth: usize,
    ) -> Result<Cow<'a, str>, DraftError> {
        let (b, w, c) = (rows(b)?, rows(w)?, rows(c)?);
        if pair_map(at) {
            let (Some((bi, bm)), Some((wi, wm)), Some((ci, cm))) =
                (paired(&b), paired(&w), paired(&c))
            else {
                return Err(conflict(at));
            };
            let keys: BTreeSet<_> = bi.iter().chain(&wi).chain(&ci).collect();
            let mut values = BTreeMap::new();
            for key in keys {
                if let Some(value) = self.value(
                    bm.get(key).map(|v| v.1),
                    wm.get(key).map(|v| v.1),
                    cm.get(key).map(|v| v.1),
                    &path(at, key)?,
                    depth,
                )? {
                    let raw_key = cm
                        .get(key)
                        .or_else(|| wm.get(key))
                        .ok_or(DraftError::Malformed)?
                        .0;
                    values.insert(key.clone(), (raw_key, value));
                }
            }
            let mut result = String::from("[");
            let mut used = BTreeSet::new();
            for key in ci.iter().chain(&wi) {
                if !used.insert(key) {
                    continue;
                }
                if let Some((raw, value)) = values.get(key) {
                    self.append(&mut result, raw.get(), at)?;
                    self.append(&mut result, value, at)?;
                }
            }
            result.push(']');
            return Ok(Cow::Owned(result));
        }
        let (Some((bi, bm)), Some((wi, wm)), Some((ci, cm))) =
            (identified(at, &b), identified(at, &w), identified(at, &c))
        else {
            return Err(conflict(at));
        };
        let keys: BTreeSet<_> = bi.iter().chain(&wi).chain(&ci).collect();
        let mut values = BTreeMap::new();
        for key in keys {
            if let Some(value) = self.value(
                bm.get(key).copied(),
                wm.get(key).copied(),
                cm.get(key).copied(),
                &path(at, key)?,
                depth,
            )? {
                values.insert(key.clone(), value);
            }
        }
        let order = order(&bi, &wi, &ci, values.keys().cloned().collect(), at)?;
        let mut result = String::from("[");
        for key in order {
            self.append(&mut result, &values[&key], at)?;
        }
        result.push(']');
        Ok(Cow::Owned(result))
    }
    fn append(&self, result: &mut String, value: &str, at: &str) -> Result<(), DraftError> {
        if result.len().saturating_add(value.len()).saturating_add(2) > MAXIMUM {
            return Err(conflict(at));
        }
        if result.len() > 1 {
            result.push(',');
        }
        result.push_str(value);
        Ok(())
    }
}
fn order(
    base: &[String],
    work: &[String],
    current: &[String],
    nodes: BTreeSet<String>,
    at: &str,
) -> Result<Vec<String>, DraftError> {
    let original: BTreeSet<_> = base.iter().collect();
    let retained = |list: &[String]| {
        list.iter()
            .filter(|id| original.contains(id) && nodes.contains(*id))
            .cloned()
            .collect::<Vec<_>>()
    };
    let baseline = retained(base);
    let work_changed = retained(work) != baseline;
    let current_changed = retained(current) != baseline;
    let mut edges: BTreeMap<String, BTreeSet<String>> = nodes
        .iter()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    let mut add = |list: &[String], all: bool| {
        let list: Vec<_> = list.iter().filter(|id| nodes.contains(*id)).collect();
        for pair in list.windows(2) {
            if all || !original.contains(pair[0]) || !original.contains(pair[1]) {
                edges.get_mut(pair[0]).unwrap().insert(pair[1].clone());
            }
        }
    };
    if !work_changed && !current_changed {
        add(base, true);
    }
    add(work, work_changed);
    add(current, current_changed);
    let mut indegree: BTreeMap<_, usize> = nodes.into_iter().map(|id| (id, 0)).collect();
    for values in edges.values() {
        for id in values {
            *indegree.get_mut(id).ok_or(DraftError::Malformed)? += 1;
        }
    }
    let mut ready: BTreeSet<_> = indegree
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut result = vec![];
    while let Some(id) = ready.pop_first() {
        result.push(id.clone());
        for next in &edges[&id] {
            let n = indegree.get_mut(next).ok_or(DraftError::Malformed)?;
            *n -= 1;
            if *n == 0 {
                ready.insert(next.clone());
            }
        }
    }
    if result.len() != indegree.len() {
        return Err(conflict(at));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn combine(base: &str, working: &str, current: &str, at: &str) -> Result<String, DraftError> {
        let raw = |s: &str| RawValue::from_string(s.into()).unwrap();
        let (base, working, current) = (raw(base), raw(working), raw(current));
        Merger {
            remaining: MAXIMUM * 4,
        }
        .value(Some(&base), Some(&working), Some(&current), at, 0)
        .map(|v| v.unwrap().into_owned())
    }
    #[test]
    fn merges_independent_identity_edits_and_retains_precise_values() {
        let result = combine(
            r#"[{"id":"a","x":1},{"id":"b","x":2}]"#,
            r#"[{"id":"a","x":3},{"id":"b","x":2}]"#,
            r#"[{"id":"a","x":1},{"id":"b","x":9007199254740993}]"#,
            "/profiles",
        )
        .unwrap();
        assert!(result.contains("9007199254740993"));
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v[0]["x"], 3);
    }
    #[test]
    fn conflicting_reorders_and_delete_edit_collisions_fail_closed() {
        assert!(matches!(
            combine(
                r#"[{"id":"a"},{"id":"b"},{"id":"c"}]"#,
                r#"[{"id":"b"},{"id":"a"},{"id":"c"}]"#,
                r#"[{"id":"a"},{"id":"c"},{"id":"b"}]"#,
                "/profiles",
            ),
            Err(DraftError::MergeConflict(_))
        ));
        assert!(matches!(
            combine(
                r#"[{"id":"a","x":1},{"id":"b"}]"#,
                r#"[{"id":"b"}]"#,
                r#"[{"id":"a","x":2},{"id":"b"}]"#,
                "/profiles",
            ),
            Err(DraftError::MergeConflict(_))
        ));
    }
    #[test]
    fn paired_customizations_merge_per_key_and_unknown_arrays_conflict() {
        let result = combine(
            r#"["a",1,"b",2]"#,
            r#"["a",3,"b",2]"#,
            r#"["a",1,"b",9007199254740993]"#,
            "/profiles/p/customization/buttonCustomizations",
        )
        .unwrap();
        assert_eq!(result, r#"["a",3,"b",9007199254740993]"#);
        assert!(matches!(
            combine("[1,2]", "[3,2]", "[1,4]", "/opaque"),
            Err(DraftError::MergeConflict(_))
        ));
    }
}
