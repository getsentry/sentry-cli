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
