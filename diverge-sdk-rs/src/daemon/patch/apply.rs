//! Applying a patch to a document.

use serde_json::Value;

use super::{Error, Kind, Operation};

/// `document` with `patch` applied, operation by operation in order;
/// the first operation that fails is the [`Error`], and the document
/// as it was is what the caller still holds, since nothing here is
/// written anywhere.
pub fn apply(document: Value, patch: &[Operation]) -> Result<Value, Error> {
    let mut document = document;
    for (index, operation) in patch.iter().enumerate() {
        document = step(document, operation).map_err(|kind| Error {
            index,
            operation: operation.name(),
            kind,
        })?;
    }
    Ok(document)
}

/// Whether the patch writes at `path` or beneath it: an `add`, a
/// `remove`, a `replace` or the `path` of a `move` or a `copy` there,
/// or the `from` of a `move` there, since a move takes away. A `test`
/// writes nothing. What a daemon asks to know whether a member was
/// touched, where touching it has an effect beyond its value — a
/// credential rewritten mints a key.
pub fn writes(patch: &[Operation], path: &str) -> bool {
    let under = |candidate: &str| candidate == path || candidate.strip_prefix(path).is_some_and(|rest| rest.starts_with('/'));
    patch.iter().any(|operation| match operation {
        Operation::Add { path, .. } | Operation::Remove { path } | Operation::Replace { path, .. } | Operation::Copy { path, .. } => under(path),
        Operation::Move { from, path } => under(from) || under(path),
        Operation::Test { .. } => false,
    })
}

/// One operation applied.
fn step(mut document: Value, operation: &Operation) -> Result<Value, Kind> {
    match operation {
        Operation::Add { path, value } => {
            add(&mut document, &tokens(path)?, path, value.clone())?;
        }
        Operation::Remove { path } => {
            remove(&mut document, &tokens(path)?, path)?;
        }
        Operation::Replace { path, value } => {
            *get_mut(&mut document, &tokens(path)?, path)? = value.clone();
        }
        Operation::Move { from, path } => {
            let from_tokens = tokens(from)?;
            let path_tokens = tokens(path)?;
            if path_tokens.len() > from_tokens.len() && path_tokens[..from_tokens.len()] == from_tokens[..] {
                return Err(Kind::Descendant {
                    from: from.clone(),
                    path: path.clone(),
                });
            }
            let value = remove(&mut document, &from_tokens, from)?;
            add(&mut document, &path_tokens, path, value)?;
        }
        Operation::Copy { from, path } => {
            let value = get(&document, &tokens(from)?, from)?.clone();
            add(&mut document, &tokens(path)?, path, value)?;
        }
        Operation::Test { path, value } => {
            if get(&document, &tokens(path)?, path)? != value {
                return Err(Kind::Test { path: path.clone() });
            }
        }
    }
    Ok(document)
}

/// The pointer's reference tokens, unescaped: none for the empty
/// pointer, else one per `/`-led part.
fn tokens(path: &str) -> Result<Vec<String>, Kind> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = path.strip_prefix('/') else {
        return Err(Kind::Pointer { path: path.to_string() });
    };
    rest.split('/').map(|token| unescape(token, path)).collect()
}

/// One token with `~1` read as `/` and `~0` as `~`, any other `~` the
/// pointer's refusal.
fn unescape(token: &str, path: &str) -> Result<String, Kind> {
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(c) = chars.next() {
        if c != '~' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('0') => out.push('~'),
            Some('1') => out.push('/'),
            _ => return Err(Kind::Pointer { path: path.to_string() }),
        }
    }
    Ok(out)
}

/// The index a token names in a list of `len`, for a place that
/// exists; `-` is none.
fn index(token: &str, len: usize, path: &str) -> Result<usize, Kind> {
    let refused = || Kind::Index { path: path.to_string() };
    if token != "0" && (token.starts_with('0') || token.starts_with('+') || token.starts_with('-')) {
        return Err(refused());
    }
    let index: usize = token.parse().map_err(|_| refused())?;
    if index >= len {
        return Err(refused());
    }
    Ok(index)
}

/// What is at the tokens.
fn get<'a>(document: &'a Value, tokens: &[String], path: &str) -> Result<&'a Value, Kind> {
    let mut at = document;
    for token in tokens {
        at = match at {
            Value::Object(members) => members.get(token).ok_or_else(|| Kind::Missing { path: path.to_string() })?,
            Value::Array(items) => &items[index(token, items.len(), path)?],
            _ => return Err(Kind::Container { path: path.to_string() }),
        };
    }
    Ok(at)
}

/// What is at the tokens, to be replaced.
fn get_mut<'a>(document: &'a mut Value, tokens: &[String], path: &str) -> Result<&'a mut Value, Kind> {
    let mut at = document;
    for token in tokens {
        at = match at {
            Value::Object(members) => members.get_mut(token).ok_or_else(|| Kind::Missing { path: path.to_string() })?,
            Value::Array(items) => {
                let index = index(token, items.len(), path)?;
                &mut items[index]
            }
            _ => return Err(Kind::Container { path: path.to_string() }),
        };
    }
    Ok(at)
}

/// `value` put at the tokens: the whole document for none; else into
/// the parent, which must exist and be an object or a list.
fn add(document: &mut Value, tokens: &[String], path: &str, value: Value) -> Result<(), Kind> {
    let Some((last, parents)) = tokens.split_last() else {
        *document = value;
        return Ok(());
    };
    match get_mut(document, parents, path)? {
        Value::Object(members) => {
            members.insert(last.clone(), value);
        }
        Value::Array(items) => {
            if last == "-" {
                items.push(value);
            } else {
                // One past the end is a place for an add, and `index`
                // refuses it: the end is asked for separately.
                let at = if *last == items.len().to_string() {
                    items.len()
                } else {
                    index(last, items.len(), path)?
                };
                items.insert(at, value);
            }
        }
        _ => return Err(Kind::Container { path: path.to_string() }),
    }
    Ok(())
}

/// What was at the tokens, taken away; the whole document is nobody's
/// to take.
fn remove(document: &mut Value, tokens: &[String], path: &str) -> Result<Value, Kind> {
    let Some((last, parents)) = tokens.split_last() else {
        return Err(Kind::Root);
    };
    match get_mut(document, parents, path)? {
        Value::Object(members) => members.remove(last).ok_or_else(|| Kind::Missing { path: path.to_string() }),
        Value::Array(items) => {
            let at = index(last, items.len(), path)?;
            Ok(items.remove(at))
        }
        _ => Err(Kind::Container { path: path.to_string() }),
    }
}
