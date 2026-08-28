---
name: deslop
description: Audit prose for LLM clichés with the deslop CLI and rewrite until clean. Use before finishing any user-facing prose (READMEs, docs, PR bodies, release notes, comments) or when asked whether text reads as AI-written.
---

# deslop

Run the audit, rewrite the flagged sentences, recheck until exit 0.

```sh
deslop --jsonl path/or/dir          # or: deslop --jsonl --text "draft"
deslop --check --quiet path/or/dir  # 0 clean, 1 matches remain
```

Loop:

1. Run `deslop --jsonl` on the file, directory, or `--text`. Directories are walked with `.gitignore` respected, picking up `md,txt,mdx,rst` (override with `--ext`).
2. For each line, rewrite `sentence` following `hint`. Restate the claim plainly instead of deleting the matched words and leaving a stub.
3. Run `deslop --check --quiet` on the same input. Exit 0 means done.
4. Otherwise repeat. Two or three passes is normal, since rewrites can trip a different pattern.

Fields per JSONL line: `path`, `pattern`, `name`, `hint`, `line`, `col`, `start`, `end`, `text`, `sentence`, `version` (schema version, currently 1). Chain patterns add `count`, and `note` appears when the detector has extra context.

Tuning: `--skip colon-triple` for technical docs where colon lists are legitimate, `--only id,id` to gate on a subset, `deslop --list-patterns --json` for the full catalogue. `deslop --agent-help` prints this loop from the binary.
