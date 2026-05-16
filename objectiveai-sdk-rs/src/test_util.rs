#[cfg(test)]
macro_rules! index_map {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut map = indexmap::IndexMap::new();
        $(map.insert($key.to_string(), $value);)*
        map
    }};
}
#[cfg(test)]
pub(crate) use index_map;
