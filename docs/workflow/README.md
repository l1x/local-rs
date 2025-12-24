**The Idea**

- Break down complex features into structured, verifiable steps rather than monolithic AI requests
- Guide the agent step-by-step with built-in checkpoints for human review

**Three-Phase Workflow**

1. **PRD (Product Requirements Document) Creation** (`create-prd.md`)
   - Define what you're building, for whom, and why
   - Feed the agent your feature description + optionally reference existing files for context
   - Output: `prds/prd-[kebab-case-feature-name]-[YYYY-MM-DD].md`

1. **Task Generation** (`generate-tasks.md`)
   - Take the PRD and break it into granular, actionable tasks
   - Use 'bd' for task tracking (tool: bd prime)
     - Beads supports hierarchical IDs for epics:
       - bd-a3f8 (Epic)
       - bd-a3f8.1 (Task)
   - Creates a implementation roadmap (bd-a3f8, bd-a3f8.1, etc.)
   - Output: tickets in beads (tool: bd prime)

1. **Iterative Implementation** (`execute-task.md`)
   - Work through tasks one task at a time
   - The task result is a pull request
   - The task runs in a Docker container
   - On merge to main executes project manager role with the task to verify task completion and close the resolved tickets

**Key Principles**

- Small, digestible chunks reduce AI errors
- Human verification at each step maintains control
- Clear progress visibility via task completion markers

```mermaid
flowchart TD
    subgraph Phase1["Phase 1: PRD Creation"]
        A[Feature Idea] --> B["create-prd.md"]
        B --> C["Define: What, Who, Why"]
        C --> D["+ Reference existing files"]
        D --> E["feature1-prd.md"]
    end

    subgraph Phase2["Phase 2: Task Generation"]
        E --> F["generate-tasks.md"]
        F --> G["Break into granular tasks"]
        G --> H["Hierarchical IDs via bd prime<br/>bd-a3f8 → bd-a3f8.1 → bd-a3f8.1.1"]
        H --> I["Tickets in Beads"]
    end

    subgraph Phase3["Phase 3: Iterative Implementation"]
        I --> J["Pick next sub-task"]
        J --> K["AI implements"]
        K --> L{"Human Review"}
        L -->|"Approve"| M["Mark complete"]
        L -->|"Revise"| K
        M --> N{"More tasks?"}
        N -->|"Yes"| J
        N -->|"No"| O["Feature Complete"]
    end

    style Phase1 fill:#f5f5f5,stroke:#9ca3af
    style Phase2 fill:#f5f5f5,stroke:#9ca3af
    style Phase3 fill:#fff7ed,stroke:#fb923c
    style E fill:#fed7aa,stroke:#f97316
    style I fill:#fed7aa,stroke:#f97316
    style L fill:#fdba74,stroke:#ea580c
    style O fill:#fb923c,stroke:#ea580c,color:#fff
```
