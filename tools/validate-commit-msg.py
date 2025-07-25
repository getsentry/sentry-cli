#!/usr/bin/env python3
"""
Validates commit messages according to Sentry's commit message guidelines.
https://develop.sentry.dev/engineering-practices/commit-messages/
"""

import re
import sys


def validate_commit_message(message):
    """
    Validates a commit message according to Sentry's format:
    <type>(<scope>): <subject>

    Returns tuple of (is_valid, error_message)
    """
    # Valid commit types
    valid_types = [
        "build",
        "ci",
        "docs",
        "feat",
        "fix",
        "perf",
        "ref",
        "style",
        "test",
        "meta",
        "license",
        "revert",
    ]

    # Skip validation for merge commits
    if message.startswith("Merge"):
        return True, None

    # Parse the first line (header)
    lines = message.strip().split("\n")

    header = lines[0].strip()

    # Pattern for the header: type(scope): subject or type: subject
    pattern = r"^(?P<type>[a-z]+)(?:\((?P<scope>[^)]+)\))?: (?P<subject>.+)$"
    match = re.match(pattern, header)

    if not match:
        return (
            False,
            "Invalid format. Must be: <type>(<scope>): <subject> or <type>: <subject>",
        )

    commit_type = match.group("type")
    scope = match.group("scope")
    subject = match.group("subject")

    # Validate type
    if commit_type not in valid_types:
        return (
            False,
            f"Invalid type '{commit_type}'. Must be one of: {', '.join(valid_types)}",
        )

    # Validate scope (if present)
    if scope and not scope.islower():
        return False, f"Scope '{scope}' must be lowercase"

    # Validate subject
    if not subject:
        return False, "Subject cannot be empty"

    # Check first letter is capitalized
    if subject[0].islower():
        return False, "Subject must start with a capital letter"

    # Check for trailing period
    if subject.endswith("."):
        return False, "Subject must not end with a period"

    # Check header length (max 70 characters)
    if len(header) > 70:
        return False, f"Header is {len(header)} characters, must be 70 or less"

    return True, None


def main():
    """Main entry point for the commit message validator."""
    # Read commit message from file (provided by git)
    if len(sys.argv) < 2:
        print("Error: No commit message file provided")
        sys.exit(1)

    commit_msg_file = sys.argv[1]

    try:
        with open(commit_msg_file, "r", encoding="utf-8") as f:
            commit_message = f.read()
    except Exception as e:
        print(f"Error reading commit message file: {e}")
        sys.exit(1)

    # Validate the commit message
    is_valid, error_msg = validate_commit_message(commit_message)

    if not is_valid:
        print(f"❌ Commit message validation failed:\n{error_msg}")
        print("\nCommit message format: <type>(<scope>): <subject>")
        print("Example: feat(api): Add new authentication endpoint")
        print(
            "\nSee https://develop.sentry.dev/engineering-practices/commit-messages/ for details"
        )
        sys.exit(1)


if __name__ == "__main__":
    main()
