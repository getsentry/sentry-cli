use crate::config::Config;

use super::releases;

pub trait DefaultTags {
    fn with_default_tags(self) -> Self;
}

impl DefaultTags for Vec<(String, String)> {
    fn with_default_tags(mut self) -> Self {
        let contains_release = self.iter().any(|(key, _)| key == "release");
        let contains_environment = self.iter().any(|(key, _)| key == "environment");
        if !contains_release {
            if let Ok(release) = releases::detect_release_name() {
                self.push(("release".into(), release));
            }
        }
        if !contains_environment {
            self.push((
                "environment".into(),
                Config::current()
                    .get_environment()
                    .filter(|e| !e.is_empty())
                    .unwrap_or("production".into()),
            ));
        }
        self
    }
}
