# deslop

Audit prose for LLM clichés from the command line. A Rust port of the "LLM cliché highlighter" tool created by Simon Willison.

I will **not** remix `deslop` core features as the intention is to remain faithful to what Simon created. You can find it live through https://tools.simonwillison.net/llm-cliche-highlighter. His repo is also available at https://github.com/simonw/tools/blob/main/llm-cliche-highlighter.html.

Nothing was changed from the original idea. This is just a rust wrapper that can be installed and runs well with your agents. You can also make it a skill by copying SKILL.md to your favorite provider.

## Install

```sh
cargo install --path .
```

## Usage

```sh
deslop draft.md                 # one line per match with a fix hint, then a summary
deslop docs/                    # walk a directory, .gitignore respected, md/txt/mdx/rst
deslop --ext md,adoc docs/      # change which extensions the walk picks up
deslop --text "Turns out ..."   # audit inline prose, no file or pipe needed
cat draft.md | deslop           # stdin works too
deslop --excerpt draft.md       # flagged sentences with 12 words of context
deslop --json draft.md          # full report with byte offsets and sentences
deslop --jsonl a.md b.md        # one JSON object per match
deslop --skip colon-triple *.md # drop a pattern that is noisy for technical docs
deslop --only ai-vocab x.md     # run a subset
deslop --check README.md        # exit 1 when anything matches (CI gate)
deslop --check --quiet README.md  # same, with no output
deslop --list-patterns          # every pattern id with its description and fix hint
deslop --agent-help             # the run, rewrite, recheck loop for agents
deslop --example | deslop       # sample text that trips every pattern
```

Text output is grep-style, `PATH:LINE:COL: PATTERN-ID  “matched text”  (note)`, followed by a `fix:` line with a one-line rewrite hint, so editors and agents can jump straight to the spot and act on it. JSON and JSONL carry the same `hint` per match plus a `version` field (schema version, currently 1) for consumers to pin. Exit codes: 0 ran, 1 matches found with `--check`, 2 bad input or unknown pattern id.

## Examples

Audit a sentence and get a fix hint per match:

```sh
$ deslop --text "Turns out the fix was simple. No config, no restart, no downtime. That's the whole point."
<text1>:1:1: turns-out  “Turns out”
    fix: Delete “turns out” and state the finding.
<text1>:1:31: no-chain  “No config, no restart, no downtime”  (3 “no” items)
    fix: Keep one “no” item or rewrite as a plain positive statement of what it is.
<text1>:1:67: whole  “That's the whole point”
    fix: State the point directly instead of announcing that it is the whole point.

<text1>: 3 matches, 3 flagged sentences, 3 chain items
    1  no-chain
    1  turns-out
    1  whole
```

Rewritten, the same idea passes clean:

```sh
$ deslop --check --text "The fix was a one-line change and needed no restart."
<text1>: 0 matches, 0 flagged sentences, 0 chain items
$ echo $?
0
```

Machine-readable output for an agent or script, one object per match:

```sh
$ deslop --jsonl --text "Turns out the fix was simple."
{"path":"<text1>","pattern":"turns-out","name":"“Turns out …”","hint":"Delete “turns out” and state the finding.","line":1,"col":1,"start":0,"end":9,"text":"Turns out","sentence":"Turns out the fix was simple.","version":1}
```

Gate a docs folder in CI, skipping a pattern that is noisy for technical writing:

```sh
deslop --check --quiet --skip colon-triple docs/ || echo "AI clichés found"
```

## Agents

`SKILL.md` is a drop-in skill for Claude Code or similar. The loop it describes: run `deslop --jsonl`, rewrite each `sentence` following its `hint`, rerun with `--check --quiet` until exit 0. `deslop --agent-help` prints the same loop from the binary.

## Patterns

38 detectors: chain patterns (`no X, no Y`, `did not X, did not Y`), stock phrases (`that's the whole point`, `sit with that`, `turns out`), structural tells (echoing sentence skeletons, stacked rhetorical questions, repeated sentence openers, colon into a triple), and the vocabulary and boilerplate catalogued in Wikipedia's [Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing). Overlapping hits collapse to one match, and each match is mapped to its containing sentence.

To add a pattern, append an entry (id, name, description, hint, finder) to `build_patterns` in `src/patterns.rs` and a few cases to the `pattern_cases` test table in `src/lib.rs`. Finder implementations live in `src/finders.rs`, sentence and window logic in `src/report.rs`.

## Tests

```sh
cargo test
```

The self-tests from the original page are ported as-is: 190 per-pattern cases plus sentence-bound, context-window, and example-text checks. `tests/cli.rs` covers directory walking, `--text`, hints, `--quiet`, and the JSON schema version.

## Contributing

While you can in fact make my rust implementation suck less, I'd advise against change its core features. You can fork this repo and make your own spin out of it, but my intention is to avoid drifting from what Simon has created.

## License

MIT
