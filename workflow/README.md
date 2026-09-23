# Workflow Convention

Every repeatable workflow lives in its own directory. The Markdown document is the source of intent and must appear before executable scripts in the directory structure.

```text
workflow/<workflow-name>/
├── <workflow-name>.md
└── scripts/
    ├── collect.ts
    └── summarize.ts
```

Use Bun and TypeScript for scripts. Keep scripts small and composable; document inputs, outputs, filters, side effects, and rollback behavior in the workflow Markdown before adding code.
