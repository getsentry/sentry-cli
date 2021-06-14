// Build parameters are used to point to the appropraite Mayhem host and API
// token credentials.
properties([
    parameters([
        string(name: 'TARGET_DOCKER_REGISTRY',
               description: 'The docker registry where the tag will be pushed'),
    ])
])

node(label: 'linux') {
  checkout scm

  def cliVersion = sh(script: "grep -R \"^version\" Cargo.toml | cut -d\" \" -f3 | cut -d'\"' -f2", returnStdout: true).trim()
  def imageTag = "${env.TARGET_DOCKER_REGISTRY}/sentry-cli"

  docker.build(imageTag, "-f Dockerfile .")
  image.push(cliVersion)
}