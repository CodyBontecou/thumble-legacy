//! Bounded, ordered JSON value edits. Untouched bytes never pass through a
//! normalized JSON value or a floating-point number representation.
use serde_json::value::RawValue;
use std::{collections::BTreeMap, fmt, ops::Range};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy)]
pub struct JsonValueEdit<'a> {
    /// RFC 6901 pointer, with canonical decimal array indices. Root replacement
    /// is deliberately excluded; an empty final key is still addressed by '/'.
    pub pointer: &'a str,
    /// Some JSON replaces/adds a value; None removes an existing value.
    pub value: Option<&'a [u8]>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonEditError {
    TooLarge,
    TooManyEdits,
    InvalidJson,
    InvalidPointer,
    MissingValue,
}
impl fmt::Display for JsonEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "JSON edit exceeds its document budget",
            Self::TooManyEdits => "too many JSON edits",
            Self::InvalidJson => "invalid or ambiguous JSON edit",
            Self::InvalidPointer => "invalid JSON edit path",
            Self::MissingValue => "JSON edit path is absent",
        })
    }
}
impl std::error::Error for JsonEditError {}

/// Apply at most 512 ordered mutations, with at most `maximum_bytes` of total
/// pointer/replacement input and output. Array indices refer to preceding edits'
/// results. Failure returns no partial document and never mutates the source.
pub fn edit_json_values(
    source: &[u8],
    edits: &[JsonValueEdit<'_>],
    maximum_bytes: usize,
) -> Result<Vec<u8>, JsonEditError> {
    if source.len() > maximum_bytes {
        return Err(JsonEditError::TooLarge);
    }
    if edits.len() > 512 {
        return Err(JsonEditError::TooManyEdits);
    }
    crate::validate_unique_json(source).map_err(|_| JsonEditError::InvalidJson)?;
    let mut budget = 0usize;
    for edit in edits {
        budget = budget
            .checked_add(edit.pointer.len())
            .and_then(|n| n.checked_add(edit.value.map_or(0, <[u8]>::len)))
            .ok_or(JsonEditError::TooLarge)?;
        if budget > maximum_bytes {
            return Err(JsonEditError::TooLarge);
        }
        pointer(edit.pointer)?;
        if let Some(value) = edit.value {
            crate::validate_unique_json(value).map_err(|_| JsonEditError::InvalidJson)?;
        }
    }
    let mut result = source.to_vec();
    for edit in edits {
        result = edit_one(&result, edit, maximum_bytes)?;
        crate::validate_unique_json(&result).map_err(|_| JsonEditError::InvalidJson)?;
    }
    Ok(result)
}
fn pointer(path: &str) -> Result<Vec<String>, JsonEditError> {
    if !path.starts_with('/') || path.len() > 4096 {
        return Err(JsonEditError::InvalidPointer);
    }
    let mut result = Vec::new();
    for token in path[1..].split('/') {
        if result.len() == 128 {
            return Err(JsonEditError::InvalidPointer);
        }
        let mut value = String::new();
        let mut chars = token.chars();
        while let Some(c) = chars.next() {
            value.push(if c == '~' {
                match chars.next() {
                    Some('0') => '~',
                    Some('1') => '/',
                    _ => return Err(JsonEditError::InvalidPointer),
                }
            } else {
                c
            });
        }
        result.push(value);
    }
    Ok(result)
}
fn index(token: &str) -> Result<usize, JsonEditError> {
    if token.is_empty()
        || !token.bytes().all(|c| c.is_ascii_digit())
        || (token.len() > 1 && token.starts_with('0'))
    {
        return Err(JsonEditError::InvalidPointer);
    }
    token.parse().map_err(|_| JsonEditError::InvalidPointer)
}
fn object(value: &RawValue) -> Result<BTreeMap<String, &RawValue>, JsonEditError> {
    serde_json::from_str(value.get()).map_err(|_| JsonEditError::InvalidPointer)
}
fn array(value: &RawValue) -> Result<Vec<&RawValue>, JsonEditError> {
    serde_json::from_str(value.get()).map_err(|_| JsonEditError::InvalidPointer)
}
fn key_eq(a: &str, b: &str) -> bool {
    a.nfc().eq(b.nfc())
}
fn range(source: &[u8], value: &RawValue) -> Result<Range<usize>, JsonEditError> {
    let start = (value.get().as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .ok_or(JsonEditError::InvalidJson)?;
    let end = start
        .checked_add(value.get().len())
        .filter(|end| *end <= source.len())
        .ok_or(JsonEditError::InvalidJson)?;
    Ok(start..end)
}
fn skip_space(source: &[u8], mut offset: usize) -> usize {
    while source.get(offset).is_some_and(u8::is_ascii_whitespace) {
        offset += 1;
    }
    offset
}
fn splice(
    source: &[u8],
    range: Range<usize>,
    bytes: &[u8],
    maximum: usize,
) -> Result<Vec<u8>, JsonEditError> {
    let size = source
        .len()
        .checked_sub(range.len())
        .and_then(|n| n.checked_add(bytes.len()))
        .filter(|n| *n <= maximum)
        .ok_or(JsonEditError::TooLarge)?;
    let mut result = Vec::with_capacity(size);
    result.extend_from_slice(&source[..range.start]);
    result.extend_from_slice(bytes);
    result.extend_from_slice(&source[range.end..]);
    Ok(result)
}
fn edit_one(
    source: &[u8],
    edit: &JsonValueEdit<'_>,
    maximum: usize,
) -> Result<Vec<u8>, JsonEditError> {
    let tokens = pointer(edit.pointer)?;
    let (last, parents) = tokens.split_last().ok_or(JsonEditError::InvalidPointer)?;
    let mut node: &RawValue =
        serde_json::from_slice(source).map_err(|_| JsonEditError::InvalidJson)?;
    for token in parents {
        node = match node.get().as_bytes().first() {
            Some(b'{') => object(node)?
                .into_iter()
                .find(|(key, _)| key_eq(key, token))
                .map(|(_, value)| value)
                .ok_or(JsonEditError::MissingValue)?,
            Some(b'[') => *array(node)?
                .get(index(token)?)
                .ok_or(JsonEditError::MissingValue)?,
            _ => return Err(JsonEditError::InvalidPointer),
        };
    }
    let parent_range = range(source, node)?;
    match node.get().as_bytes().first() {
        Some(b'{') => {
            let mut fields: Vec<_> = object(node)?
                .into_iter()
                .map(|(key, value)| Ok((key, range(source, value)?)))
                .collect::<Result<_, JsonEditError>>()?;
            fields.sort_by_key(|(_, range)| range.start);
            if let Some(position) = fields.iter().position(|(key, _)| key_eq(key, last)) {
                let target = fields[position].1.clone();
                if let Some(value) = edit.value {
                    return splice(source, target, value, maximum);
                }
                let preceding = if position == 0 {
                    parent_range.start + 1
                } else {
                    skip_space(source, fields[position - 1].1.end) + 1
                };
                let entry_start = skip_space(source, preceding);
                return remove(
                    source,
                    entry_start..target.end,
                    position,
                    fields.len(),
                    position.checked_sub(1).map(|i| fields[i].1.end),
                    maximum,
                );
            }
            let Some(value) = edit.value else {
                return Err(JsonEditError::MissingValue);
            };
            let offset = fields.last().map_or(parent_range.start + 1, |(_, r)| r.end);
            let mut addition = Vec::new();
            if !fields.is_empty() {
                addition.push(b',');
            }
            addition.extend(serde_json::to_vec(last).map_err(|_| JsonEditError::InvalidPointer)?);
            addition.push(b':');
            addition.extend_from_slice(value);
            splice(source, offset..offset, &addition, maximum)
        }
        Some(b'[') => {
            let values = array(node)?;
            let position = if last == "-" {
                values.len()
            } else {
                index(last)?
            };
            if position > values.len() {
                return Err(JsonEditError::MissingValue);
            }
            if let Some(target) = values.get(position) {
                let target = range(source, target)?;
                if let Some(value) = edit.value {
                    return splice(source, target, value, maximum);
                }
                let previous_end = position
                    .checked_sub(1)
                    .map(|i| range(source, values[i]).map(|r| r.end))
                    .transpose()?;
                remove(
                    source,
                    target,
                    position,
                    values.len(),
                    previous_end,
                    maximum,
                )
            } else {
                let Some(value) = edit.value else {
                    return Err(JsonEditError::MissingValue);
                };
                let offset = values
                    .last()
                    .map(|v| range(source, v).map(|r| r.end))
                    .transpose()?
                    .unwrap_or(parent_range.start + 1);
                let mut addition = Vec::new();
                if !values.is_empty() {
                    addition.push(b',');
                }
                addition.extend_from_slice(value);
                splice(source, offset..offset, &addition, maximum)
            }
        }
        _ => Err(JsonEditError::InvalidPointer),
    }
}
fn remove(
    source: &[u8],
    target: Range<usize>,
    position: usize,
    count: usize,
    previous_end: Option<usize>,
    maximum: usize,
) -> Result<Vec<u8>, JsonEditError> {
    let range = if position + 1 < count {
        let comma = skip_space(source, target.end);
        if source.get(comma) != Some(&b',') {
            return Err(JsonEditError::InvalidJson);
        }
        target.start..comma + 1
    } else if let Some(previous_end) = previous_end {
        let comma = skip_space(source, previous_end);
        if source.get(comma) != Some(&b',') {
            return Err(JsonEditError::InvalidJson);
        }
        comma..target.end
    } else {
        target
    };
    splice(source, range, &[], maximum)
}
