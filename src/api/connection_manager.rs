pub(super) struct CurlConnectionManager;

impl r2d2::ManageConnection for CurlConnectionManager {
    type Connection = curl::easy::Easy;
    type Error = curl::Error;

    fn connect(&self) -> Result<curl::easy::Easy, curl::Error> {
        Ok(curl::easy::Easy::new())
    }

    fn is_valid(&self, _conn: &mut curl::easy::Easy) -> Result<(), curl::Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut curl::easy::Easy) -> bool {
        false
    }
}
