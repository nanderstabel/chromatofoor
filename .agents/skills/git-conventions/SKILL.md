---
name: git-conventions
description: Rules and guidelines for writing structured git commits using conventional commit types.
---

# Git Commit Conventions

This skill defines the Git commit message standard for this repository, based on Conventional Commits and the BAVAGA commit types cheatsheet.

## Format

Commit messages MUST follow this format:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

- **Type**: Must be one of the 11 conventional types listed below.
- **Scope**: (Optional but highly recommended) The area of the codebase the change applies to (e.g., `generator`, `framework`, `examples`, `property`, `tenancy`).
- **Description**: A short, imperative summary of the change (lowercase, no period at the end).

---

## Commit Types

| Type | When to Use | Examples |
| :--- | :--- | :--- |
| **`feat`** | A new feature or capability introduced to the codebase. | `feat(generator): add support for process managers` |
| **`fix`** | A bug fix or correction to existing behavior. | `fix(framework): resolve race condition in event delivery` |
| **`chore`** | Maintenance, routine tasks, configuration tweaks, or repository administration (files/folders movement, updating gitignore, formatting build scripts) that do *not* modify source code or tests. | `chore(repo): update gitignore to ignore reference folders` |
| **`perf`** | Code changes focused specifically on improving execution performance. | `perf(generator): cache template compilation results` |
| **`docs`** | Documentation-only changes (adding/updating READMEs, inline docstrings, `.md` files). | `docs(readme): add setup instructions for examples` |
| **`build`** | Changes affecting build configuration, package managers, or external production dependencies (e.g., modifying `Cargo.toml`). | `build(deps): update tokio dependency version` |
| **`revert`** | Reverting a previous commit. | `revert: "feat(generator): add support for process managers"` |
| **`style`** | Code style changes (whitespace, formatting, missing semicolons) that do *not* affect the meaning or logic of the code. | `style(generator): run cargo fmt on main.rs` |
| **`refactor`** | Improving code structure or organization without changing its external behavior or fixing bugs (e.g. variable renaming, extracting helper functions). | `refactor(domain): simplify aggregate state initialization` |
| **`ci`** | Updates to CI/CD files and scripts (e.g., GitHub Actions workflows). | `ci(github): add cargo test step to pull request check` |
| **`test`** | Adding new tests or correcting/updating existing tests. | `test(generator): add unit tests for workspace generator` |

---

## Best Practices

1. **Imperative Mood**: Use the imperative mood in the description (e.g. `feat: add feature` instead of `feat: added feature` or `feat: adds feature`).
2. **Short & Sweet**: Keep the first line under 50 characters if possible, and never exceed 72 characters.
3. **Breaking Changes**: Mark breaking changes by appending a `!` after the type/scope (e.g. `feat(generator)!: drop support for legacy YAML structure`) or in the footer as `BREAKING CHANGE: <description>`.
