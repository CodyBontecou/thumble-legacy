//! Apply a typed projection's changes without re-encoding opaque source data.
//! This is an in-memory transform, not schema validation or an authority commit.
use crate::{edit_json_values, validate_unique_json, JsonEditError, JsonValueEdit};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap, fmt};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMergeError {
    TooLarge,
    TooComplex,
    InvalidJson,
    InvalidProjection,
    OpaqueConflict,
    Edit(JsonEditError),
}
impl fmt::Display for ProjectionMergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "projection merge exceeds its byte budget",
            Self::TooComplex => "projection merge exceeds its traversal budget",
            Self::InvalidJson => "invalid or ambiguous projection merge JSON",
            Self::InvalidProjection => "projection collection identities are invalid",
            Self::OpaqueConflict => "projection change would overwrite opaque source data",
            Self::Edit(_) => "projection edit could not be applied",
        })
    }
}
impl std::error::Error for ProjectionMergeError {}

/// `before` and `after` must be projections made by the same typed codec, with
/// `before` derived from this exact source/revision. Only their changed fields
/// are applied. Unknown source values retain their original JSON spelling.
/// Object arrays align by unique string `id` (UUID spelling is canonicalized
/// for matching only). Changed arrays without identities reject opaque data
/// rather than guessing which item owns it. Clearing a container with unknown
/// children, or adding a field over non-null unprojected data, also rejects.
/// UUID-shaped map keys with case aliases require a schema-aware adapter;
/// this transform rejects changes there instead of adding a shadow owner.
///
/// Each input/output is bounded by `maximum_bytes`. Failure returns no partial
/// result. The caller still owns codec/schema validation, identity/output guards,
/// source-projection provenance, CAS, backups, and persistence.
pub fn merge_json_projection(
    source: &[u8],
    before: &[u8],
    after: &[u8],
    maximum_bytes: usize,
) -> Result<Vec<u8>, ProjectionMergeError> {
    let original = source;
    for input in [source, before, after] {
        if input.len() > maximum_bytes {
            return Err(ProjectionMergeError::TooLarge);
        }
        validate_unique_json(input).map_err(|_| ProjectionMergeError::InvalidJson)?;
    }
    let raw = |data| serde_json::from_slice::<&RawValue>(data);
    let source = raw(source).map_err(|_| ProjectionMergeError::InvalidJson)?;
    let before = raw(before).map_err(|_| ProjectionMergeError::InvalidJson)?;
    let after = raw(after).map_err(|_| ProjectionMergeError::InvalidJson)?;
    let mut merger = Merger {
        maximum: maximum_bytes,
        remaining: maximum_bytes.saturating_mul(4),
    };
    let merged = merger.merge(source, before, after, 0)?;
    // RawValue borrows the value itself, excluding outer document whitespace.
    // Preserve that framing even for a no-op or a changed root container.
    let start = source.get().as_ptr() as usize - original.as_ptr() as usize;
    let end = start + source.get().len();
    let size = original
        .len()
        .checked_sub(source.get().len())
        .and_then(|size| size.checked_add(merged.len()))
        .filter(|size| *size <= maximum_bytes)
        .ok_or(ProjectionMergeError::TooLarge)?;
    let mut output = Vec::with_capacity(size);
    output.extend_from_slice(&original[..start]);
    output.extend_from_slice(merged.as_bytes());
    output.extend_from_slice(&original[end..]);
    Ok(output)
}

struct Field<'a> {
    key: String,
    value: &'a RawValue,
}
type Fields<'a> = BTreeMap<String, Field<'a>>;
fn fields(value: &RawValue) -> Result<Fields<'_>, ProjectionMergeError> {
    let values: BTreeMap<String, &RawValue> =
        serde_json::from_str(value.get()).map_err(|_| ProjectionMergeError::InvalidJson)?;
    Ok(values
        .into_iter()
        .map(|(key, value)| (key.nfc().collect(), Field { key, value }))
        .collect())
}
fn values(value: &RawValue) -> Result<Vec<&RawValue>, ProjectionMergeError> {
    serde_json::from_str(value.get()).map_err(|_| ProjectionMergeError::InvalidJson)
}
fn kind(value: &RawValue) -> u8 {
    value.get().as_bytes()[0]
}
fn identity(value: &RawValue) -> Result<Option<String>, ProjectionMergeError> {
    if kind(value) != b'{' {
        return Ok(None);
    }
    let object = fields(value)?;
    let Some(field) = object.get("id") else {
        return Ok(None);
    };
    let Ok(id) = serde_json::from_str::<String>(field.value.get()) else {
        return Ok(None);
    };
    Ok(Some(
        uuid::Uuid::parse_str(&id).map_or(id, |id| id.to_string()),
    ))
}
fn identities(
    items: &[&RawValue],
) -> Result<Option<BTreeMap<String, usize>>, ProjectionMergeError> {
    let mut result = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let Some(id) = identity(item)? else {
            return Ok(None);
        };
        if result.insert(id, index).is_some() {
            return Err(ProjectionMergeError::InvalidProjection);
        }
    }
    Ok(Some(result))
}
fn pointer(key: &str) -> String {
    format!("/{}", key.replace('~', "~0").replace('/', "~1"))
}

struct Merger {
    maximum: usize,
    remaining: usize,
}
impl Merger {
    fn visit(&mut self, depth: usize) -> Result<(), ProjectionMergeError> {
        if depth > 128 || self.remaining == 0 {
            return Err(ProjectionMergeError::TooComplex);
        }
        self.remaining -= 1;
        Ok(())
    }
    fn merge<'a>(
        &mut self,
        source: &'a RawValue,
        before: &RawValue,
        after: &RawValue,
        depth: usize,
    ) -> Result<Cow<'a, str>, ProjectionMergeError> {
        self.visit(depth)?;
        if before.get() == after.get() || source.get() == after.get() {
            return Ok(Cow::Borrowed(source.get()));
        }
        match (kind(source), kind(before), kind(after)) {
            (b'{', b'{', b'{') => self.object(source, before, after, depth + 1),
            (b'[', b'[', b'[') => self.array(source, before, after, depth + 1),
            _ => {
                if self.opaque(source, before, depth + 1)? {
                    return Err(ProjectionMergeError::OpaqueConflict);
                }
                Ok(Cow::Owned(after.get().into()))
            }
        }
    }
    fn object<'a>(
        &mut self,
        source: &'a RawValue,
        before: &RawValue,
        after: &RawValue,
        depth: usize,
    ) -> Result<Cow<'a, str>, ProjectionMergeError> {
        let original = fields(source)?;
        let old = fields(before)?;
        let new = fields(after)?;
        let mut edits: Vec<(String, Option<String>)> = Vec::new();
        for (key, previous) in &old {
            if !new.contains_key(key) {
                if let Some(field) = self.source_field(&original, key, depth)? {
                    if self.opaque(field.value, previous.value, depth)? {
                        return Err(ProjectionMergeError::OpaqueConflict);
                    }
                    edits.push((pointer(&field.key), None));
                }
            }
        }
        for (key, next) in &new {
            self.visit(depth)?;
            let previous = old.get(key);
            if previous.is_some_and(|field| field.value.get() == next.value.get()) {
                continue;
            }
            let original = self.source_field(&original, key, depth)?;
            let replacement = match (original, previous) {
                (Some(field), Some(previous)) => {
                    let replacement = self.merge(field.value, previous.value, next.value, depth)?;
                    if replacement.as_ref() == field.value.get() {
                        continue;
                    }
                    replacement.into_owned()
                }
                (Some(field), None) if field.value.get() == next.value.get() => continue,
                (Some(field), None) if field.value.get() != "null" => {
                    return Err(ProjectionMergeError::OpaqueConflict);
                }
                _ => next.value.get().into(),
            };
            edits.push((
                pointer(original.map_or(&next.key, |field| &field.key)),
                Some(replacement),
            ));
        }
        if edits.is_empty() {
            return Ok(Cow::Borrowed(source.get()));
        }
        let edits: Vec<_> = edits
            .iter()
            .map(|(path, value)| JsonValueEdit {
                pointer: path,
                value: value.as_ref().map(|value| value.as_bytes()),
            })
            .collect();
        let output = edit_json_values(source.get().as_bytes(), &edits, self.maximum)
            .map_err(ProjectionMergeError::Edit)?;
        String::from_utf8(output)
            .map(Cow::Owned)
            .map_err(|_| ProjectionMergeError::InvalidJson)
    }
    fn source_field<'a, 'b>(
        &mut self,
        original: &'b Fields<'a>,
        key: &str,
        depth: usize,
    ) -> Result<Option<&'b Field<'a>>, ProjectionMergeError> {
        if let Ok(id) = uuid::Uuid::parse_str(key) {
            // UUID-valued collection IDs have explicit identity semantics.
            // Arbitrary JSON map keys do not: never guess a codec's alias rule.
            for candidate in original.keys() {
                self.visit(depth)?;
                if candidate != key
                    && uuid::Uuid::parse_str(candidate).is_ok_and(|value| value == id)
                {
                    return Err(ProjectionMergeError::OpaqueConflict);
                }
            }
        }
        Ok(original.get(key))
    }
    fn array<'a>(
        &mut self,
        source: &'a RawValue,
        before: &RawValue,
        after: &RawValue,
        depth: usize,
    ) -> Result<Cow<'a, str>, ProjectionMergeError> {
        let original = values(source)?;
        let old = values(before)?;
        let new = values(after)?;
        let (original_ids, old_ids, new_ids) =
            (identities(&original)?, identities(&old)?, identities(&new)?);
        let mut output = String::from("[");
        if let (Some(original_ids), Some(old_ids), Some(new_ids)) = (original_ids, old_ids, new_ids)
        {
            for (id, &index) in &original_ids {
                let Some(&old_index) = old_ids.get(id) else {
                    return Err(ProjectionMergeError::OpaqueConflict);
                };
                if !new_ids.contains_key(id)
                    && self.opaque(original[index], old[old_index], depth)?
                {
                    return Err(ProjectionMergeError::OpaqueConflict);
                }
            }
            for item in &new {
                let id = identity(item)?.ok_or(ProjectionMergeError::InvalidProjection)?;
                let value = match (original_ids.get(&id), old_ids.get(&id)) {
                    (Some(&index), Some(&old_index)) => {
                        self.merge(original[index], old[old_index], item, depth)?
                    }
                    (Some(_), None) => return Err(ProjectionMergeError::OpaqueConflict),
                    _ => Cow::Borrowed(item.get()),
                };
                self.append(&mut output, &value)?;
            }
        } else {
            if self.opaque(source, before, depth)? {
                return Err(ProjectionMergeError::OpaqueConflict);
            }
            if original.len() != old.len() || old.len() != new.len() {
                return Ok(Cow::Owned(after.get().into()));
            }
            for ((source, before), after) in original.iter().zip(&old).zip(&new) {
                let value = self.merge(source, before, after, depth)?;
                self.append(&mut output, &value)?;
            }
        }
        if output.len() >= self.maximum {
            return Err(ProjectionMergeError::TooLarge);
        }
        output.push(']');
        Ok(Cow::Owned(output))
    }
    fn append(&self, array: &mut String, value: &str) -> Result<(), ProjectionMergeError> {
        let size = array
            .len()
            .checked_add(usize::from(array.len() > 1))
            .and_then(|size| size.checked_add(value.len()))
            .and_then(|size| size.checked_add(1))
            .filter(|size| *size <= self.maximum)
            .ok_or(ProjectionMergeError::TooLarge)?;
        array.reserve(size - array.len());
        if array.len() > 1 {
            array.push(',');
        }
        array.push_str(value);
        Ok(())
    }
    fn opaque(
        &mut self,
        source: &RawValue,
        projection: &RawValue,
        depth: usize,
    ) -> Result<bool, ProjectionMergeError> {
        self.visit(depth)?;
        if source.get() == projection.get() {
            return Ok(false);
        }
        match (kind(source), kind(projection)) {
            (b'{', b'{') => {
                let known = fields(projection)?;
                for (key, field) in fields(source)? {
                    let Some(previous) = known.get(&key) else {
                        return Ok(true);
                    };
                    if self.opaque(field.value, previous.value, depth + 1)? {
                        return Ok(true);
                    }
                }
            }
            (b'[', b'[') => {
                let original = values(source)?;
                let projected = values(projection)?;
                if let (Some(original_ids), Some(known_ids)) =
                    (identities(&original)?, identities(&projected)?)
                {
                    for (id, index) in original_ids {
                        let Some(&known_index) = known_ids.get(&id) else {
                            return Ok(true);
                        };
                        if self.opaque(original[index], projected[known_index], depth + 1)? {
                            return Ok(true);
                        }
                    }
                } else {
                    if original.len() > projected.len() {
                        return Ok(true);
                    }
                    for (source, known) in original.iter().zip(&projected) {
                        if self.opaque(source, known, depth + 1)? {
                            return Ok(true);
                        }
                    }
                }
            }
            (b'{', _) => return Ok(!fields(source)?.is_empty()),
            (b'[', _) => return Ok(!values(source)?.is_empty()),
            _ => (),
        }
        Ok(false)
    }
}
