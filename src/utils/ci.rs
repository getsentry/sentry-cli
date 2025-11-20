use std::env;

/// Detects if the current environment is a CI environment by checking common CI environment variables.
///
/// This checks environment variables that are commonly set by CI providers like:
/// - GitHub Actions
/// - GitLab CI
/// - Jenkins
/// - CircleCI
/// - Travis CI
/// - Bitbucket Pipelines
/// - And many others
///
/// Based on: https://github.com/getsentry/sentry-android-gradle-plugin/blob/15068f51fee344c00efdbec5a172297be4719b86/plugin-build/src/main/kotlin/io/sentry/android/gradle/util/CI.kt#L9
pub fn is_ci() -> bool {
    // Check common CI environment variables
    env::var("CI").is_ok()
        || env::var("CONTINUOUS_INTEGRATION").is_ok()
        || env::var("BUILD_NUMBER").is_ok()
        || env::var("JENKINS_URL").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("TRAVIS").is_ok()
        || env::var("BITBUCKET_BUILD_NUMBER").is_ok()
        || env::var("TEAMCITY_VERSION").is_ok()
        || env::var("BUILDKITE").is_ok()
        || env::var("HUDSON_URL").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_ci_with_ci_var() {
        env::set_var("CI", "true");
        assert!(is_ci());
        env::remove_var("CI");
    }

    #[test]
    fn test_is_ci_with_github_actions() {
        env::set_var("GITHUB_ACTIONS", "true");
        assert!(is_ci());
        env::remove_var("GITHUB_ACTIONS");
    }

    #[test]
    fn test_is_not_ci() {
        // Clear all CI-related env vars
        let ci_vars = [
            "CI",
            "CONTINUOUS_INTEGRATION",
            "BUILD_NUMBER",
            "JENKINS_URL",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "CIRCLECI",
            "TRAVIS",
            "BITBUCKET_BUILD_NUMBER",
            "TEAMCITY_VERSION",
            "BUILDKITE",
            "HUDSON_URL",
        ];

        for var in &ci_vars {
            env::remove_var(var);
        }

        assert!(!is_ci());
    }
}
