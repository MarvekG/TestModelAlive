# Repository Rules

- Do not create commits directly on the `main` branch. Create and commit changes on a feature or fix branch, then merge through the repository's review workflow.
- For GitHub SSH operations, use the repository-local key: `GIT_SSH_COMMAND="ssh -i id_ed25519 -o IdentitiesOnly=yes" git <command>`.
