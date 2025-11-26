use std::env;

/// Detects if the current environment is a CI environment by checking common CI environment variables.
///
/// This checks environment variables that are commonly set by CI providers like:
/// - GitHub Actions
/// - GitLab CI
/// - Jenkins / Hudson
/// - CircleCI
/// - Travis CI
/// - TeamCity
/// - Bamboo
/// - Bitrise
/// - GoCD
/// - Azure Pipelines
/// - Buildkite
///
/// Based on: https://github.com/getsentry/sentry-android-gradle-plugin/blob/15068f51fee344c00efdbec5a172297be4719b86/plugin-build/src/main/kotlin/io/sentry/android/gradle/util/CI.kt#L9
pub fn is_ci() -> bool {
    env::var("CI").is_ok()
        || env::var("JENKINS_URL").is_ok()
        || env::var("HUDSON_URL").is_ok()
        || env::var("TEAMCITY_VERSION").is_ok()
        || env::var("CIRCLE_BUILD_URL").is_ok()
        || env::var("bamboo_resultsUrl").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("TRAVIS_JOB_ID").is_ok()
        || env::var("BITRISE_BUILD_URL").is_ok()
        || env::var("GO_SERVER_URL").is_ok()
        || env::var("TF_BUILD").is_ok()
        || env::var("BUILDKITE").is_ok()
}
