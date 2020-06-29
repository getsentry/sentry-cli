use mockito::server_url;
use std::collections::HashMap;

pub fn get_base_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert(String::from("SENTRY_URL"), server_url());
    env.insert(String::from("SENTRY_AUTH_TOKEN"), String::from("lolnope"));
    env.insert(String::from("SENTRY_ORG"), String::from("wat-org"));
    env.insert(String::from("SENTRY_PROJECT"), String::from("wat-project"));
    env
}
