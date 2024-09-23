use std::convert::Infallible;

pub(super) struct CurlConnectionManager;

impl r2d2::ManageConnection for CurlConnectionManager {
    type Connection = curl::easy::Easy;
    type Error = Infallible;

    fn connect(&self) -> Result<curl::easy::Easy, Infallible> {
        Ok(curl::easy::Easy::new())
    }

    fn is_valid(&self, _conn: &mut curl::easy::Easy) -> Result<(), Infallible> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut curl::easy::Easy) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2::ManageConnection;

    #[test]
    fn test_connect() {
        let manager = CurlConnectionManager;

        // Just make sure the connection can be established without panic.
        manager.connect().unwrap();
    }

    #[test]
    fn test_is_valid() {
        let manager = CurlConnectionManager;
        assert_eq!(manager.is_valid(&mut curl::easy::Easy::new()), Ok(()));
    }

    #[test]
    fn test_has_broken() {
        let manager = CurlConnectionManager;
        assert!(!manager.has_broken(&mut curl::easy::Easy::new()));
    }
}
