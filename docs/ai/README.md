# AI Docs and Status Reports

This folder contains AI-authored guides, status reports, research notes, and similar write-ups. To keep the root tidy and make discovery easy, all such documents must live here and be named with a date prefix.

Policy:
- Location: `docs/ai/`
- Filename prefix: `YYYY-MM-DD-`
- Remainder of the filename: short, kebab-case summary of the topic
- Example: `2025-11-01-mise-task-catalog.md`, `2025-11-01-status-weekly.md`

Scope (what belongs here):
- Guides produced by AI that were not explicitly requested as permanent top-level docs
- Status reports and experiment logs
- Research notes, explorations, and alternatives analyses

Exceptions:
- Maintainer-requested or canonical docs continue to live alongside the component they document (e.g., `DEVELOPMENT_GUIDE.md`, `TOOLING_MISE.md`).
- Existing documents remain where they are unless explicitly moved in a follow-up change.

Recommended document template:
```
# <Concise Title>

- Date: YYYY-MM-DD
- Author: AI-assisted (Junie) + human reviewer (add handle)
- Tags: <tag-1>, <tag-2>

## Summary
A few sentences on what this document covers and key decisions or findings.

## Details
Longer explanation, steps taken, commands used, alternatives considered, etc.

## Outcomes / Next steps
- Outcome 1
- Outcome 2
- Next: <todo or link to issue>
```

Tips:
- Use today’s date in your local time when creating a new file (today is 2025-11-01 by the local clock for this session).
- Keep filenames short and descriptive; details go in the document body.
- Link out to permanent docs where appropriate, rather than duplicating content.
