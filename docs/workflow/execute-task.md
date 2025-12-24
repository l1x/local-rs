<!--file:execute-task.md-->

# Execute Task

## Objective

Implement a single, atomic task defined in a Beads ticket, ensuring code quality and verification before completion.

## Input

- **Ticket ID**: The `bd` ticket ID to execute (e.g., `bd-a3f8.1`).

## Process

1.  **Claim Ticket**
    - Read the ticket details: `bd show <id>`
    - Mark as in progress: `bd update <id> --status=in_progress`

2.  **Context Loading**
    - Read the parent Epic or PRD if referenced to understand the broader scope.
    - Explore relevant codebase sections using `grep` or `ls`.

3.  **Implementation (TDD)**
    - **Create Test**: Write a failing test case that reproduces the requirement or bug.
    - **Implement**: Write the minimal code necessary to pass the test.
    - **Refactor**: Clean up code while keeping tests green.

4.  **Verification**
    - Run unit tests: `mise run unit-tests`
    - Run integration tests (if applicable): `mise run integration-tests`
    - Run full verification: `mise run verify` (includes linting)

5.  **Completion**
    - If verification fails, fix issues and repeat Step 4.
    - If verification passes:
      - Close the ticket: `bd close <id>`
      - Output a summary of changes made.

## Constraints

- **Scope**: Focus ONLY on the specified ticket. Do not implement extra features.
- **Quality**: Code must compile and pass `mise run verify`.
- **Style**: Follow project patterns (see `CLAUDE.md`).
- **Atomic**: If the task is too large, stop and request to split it into smaller tickets.

## Error Handling

- If you encounter a blocker (missing dependency, unclear requirement), add a comment to the ticket (`bd comment <id> "Blocker: ..."`) and stop.
