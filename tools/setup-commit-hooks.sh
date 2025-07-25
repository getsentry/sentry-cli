#!/bin/bash
# Script to set up commit message validation hooks for sentry-cli

set -e

echo "Setting up commit hooks for sentry-cli..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "❌ pre-commit is not installed."
    echo ""
    echo "Please install pre-commit using one of these methods:"
    echo "  - pip install pre-commit"
    echo "  - brew install pre-commit (macOS)"
    echo "  - pipx install pre-commit"
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Install the git hooks
echo "Installing pre-commit hooks..."
pre-commit install
pre-commit install --hook-type commit-msg

# Always set up the commit message template
echo "Setting up commit message template..."
git config commit.template .gitmessage
echo "✅ Commit message template configured"

echo ""
echo "✅ Setup complete!"
echo ""
echo "Your commits will now be validated against Sentry's commit message format."
echo "Format: <type>(<scope>): <subject>"
echo "Example: feat(cli): Add new authentication feature"
echo ""
echo "For more details: https://develop.sentry.dev/engineering-practices/commit-messages/"
