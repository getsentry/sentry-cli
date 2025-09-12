# Claude Code Agent Guidelines

This document provides guidance for you as the Claude Code agent on how to work with this project's context rules system.

## Rule Location and Structure

This project uses **Cursor's Context Rules** system located in `.cursor/rules/`. **ALWAYS** read and load the content of relevant rules from this directory into context when working on the codebase.

### Discovering Available Rules

Read all files in the `.cursor/rules/` directory to discover what rules are available. Each rule file uses the `.mdc` extension (Markdown with frontmatter).

## Rule File Format

Each rule file uses **Markdown with YAML frontmatter**:

```markdown
---
description: Brief description of what this rule covers
globs: file,patterns,**/*,that,trigger,this,rule
alwaysApply: false
---

# Rule Title

Content of the rule in Markdown format...
```

### Frontmatter Properties

- **`description`**: A concise explanation of what the rule covers
- **`globs`**: Comma-separated file patterns that trigger this rule (uses glob syntax)
- **`alwaysApply`**: Boolean indicating if you should always apply this rule regardless of file context

## Using Rules as Claude Code

### Mandatory Rule Loading

You **MUST**:

1. Read all rule files in `.cursor/rules/` to understand available rules
2. Load and follow the patterns and conventions defined in applicable rules
3. Apply all rules that have `alwaysApply: true` to every operation
4. Load context-specific rules based on the files you're modifying (matching globs)

### When to Load Rules

Load rules in these scenarios:

- **File modifications**: Check globs in rule frontmatter to determine which rules apply to the files being changed
- **New feature development**: Load relevant domain-specific rules based on file patterns and descriptions
- **Code reviews**: Apply project standards and patterns from applicable rules
- **Planning tasks**: Consult rules for architectural guidance and best practices

### Rule Selection Strategy

1. **Always apply** rules with `alwaysApply: true`
2. **Match file patterns** - load rules whose globs match the files you're working with
3. **Domain context** - load language/technology-specific rules based on descriptions and file patterns
4. **Multiple rules** - load and apply multiple rules simultaneously when they apply to your current task

### Reading Rules Into Context

Access the `.cursor/rules/` directory and read rule file contents. Parse the frontmatter to determine:

- Whether the rule applies to your current task (`alwaysApply` or matching `globs`)
- What the rule covers based on the `description`
- Which files trigger the rule based on `globs` patterns

## Important Notes

- **Do NOT duplicate rule content** in your responses - reference and follow the rules instead
- **Always check all rules** - examine the directory contents to ensure you don't miss any rules
- **Follow rule hierarchies** - general project rules combined with specific technology rules
- **Use rules for consistency** - ensure all code changes align with established patterns
- **Parse frontmatter carefully** - use the metadata to determine rule applicability

Treat these rules as **mandatory guidance** that you must follow for all code changes and development activities within this project.

# Code Formatting

**ALWAYS** run `cargo fmt` before committing any Rust code changes to ensure consistent formatting across the codebase.
