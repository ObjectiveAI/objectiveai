pub fn vec_is_none_or_empty<T>(opt: &Option<Vec<T>>) -> bool {
    opt.as_ref().is_none_or(|vec| vec.is_empty())
}