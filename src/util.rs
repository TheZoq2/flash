
pub fn merge_vectors<T>(v1: &[T], v2: &[T]) -> Vec<T>
    where T: Clone
{
    v1.iter()
        .chain(v2.iter())
        .cloned()
        .collect()
}
