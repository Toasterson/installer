# Documentation Guidelines

This repository uses a simple convention for AI-authored guides, status reports, research notes, and similar write‑ups to keep the top-level clean and make these documents easy to discover.

Policy (effective 2025-11-01):
- Location: place AI-authored guides and status reports under `docs/ai/`
- Filename prefix: start filenames with a date in the format `YYYY-MM-DD-`
- Naming: use short, kebab-case summaries after the date prefix
- Examples:
  - `2025-11-01-mise-task-catalog.md`
  - `2025-11-01-weekly-status.md`
  - `2025-11-01-research-dev-vm-options.md`

Scope (what belongs in `docs/ai/`):
- Guides produced by AI that were not explicitly requested as permanent, canonical docs
- Status updates and progress reports
- Experiment logs, research notes, RFC drafts, alternatives analysis, and related write-ups

Exceptions:
- Maintainer-requested or canonical documentation should continue to live alongside the components they document (e.g., `DEVELOPMENT_GUIDE.md`, `TOOLING_MISE.md`).
- Existing historical documents remain in their current locations unless a dedicated follow-up change moves them.

Date source:
- Use the local date at the time of creation when prefixing filenames (for this session, the local date is 2025-11-01). Time-of-day is not required; only `YYYY-MM-DD`.

Recommended template for AI-authored docs:
```
# <Concise Title>

- Date: YYYY-MM-DD
- Author: AI-assisted (Junie) + human reviewer (add handle)
- Tags: <tag-1>, <tag-2>

## Summary
Brief overview of intent, key decisions, and outcomes.

## Details
Steps taken, rationale, alternatives considered, and command snippets if applicable.

## Outcomes / Next steps
- Outcome 1
- Outcome 2
- Next: <todo or link to issue>
```

Rationale:
- Keeps the repository root and component directories focused on canonical docs
- Makes exploratory and progress documents easy to find under a single tree
- Date prefix provides natural chronological ordering and avoids filename collisions
