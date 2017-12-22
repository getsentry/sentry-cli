pub fn invert_result<T, E>(res: Result<Option<T>, E>) -> Option<Result<T, E>> {
    match res {
        Ok(Some(batch)) => Some(Ok(batch)),
        Err(err) => Some(Err(err)),
        Ok(None) => None,
    }
}
