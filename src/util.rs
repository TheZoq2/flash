
pub fn merge_vectors<T>(v1: &Vec<T>, v2: &Vec<T>) -> Vec<T>
    where T: Clone
{
    v1.iter()
        .chain(v2.iter())
        .map(|v| v.clone())
        .collect()
}
