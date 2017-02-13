class SentryCli < Formula
  desc "This is a Sentry command-line client for some generic tasks."
  homepage "https://github.com/getsentry/sentry-cli"
  url "https://github.com/getsentry/sentry-cli/releases/download/0.23.0/sentry-cli-Darwin-x86_64"
  version "0.23.0"
  sha256 "d093c3bf6c6c471f7b03f1e58ecd40f79b0a55c333395cad56c5f7ce05f2a75f"

  def install
    mv "sentry-cli-Darwin-x86_64", "sentry-cli"
    bin.install "sentry-cli"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sentry-cli --version").chomp
  end
end
