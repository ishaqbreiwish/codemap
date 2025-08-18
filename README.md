# codemap

Rust CLI that onboards you to any repo:

- Builds a code map (files, functions, hashes).
- Merges updates incrementally (function-level diffs).
- LLM-powered project brief + ranked entry points (optional).
- Heuristic fallback works offline.

## Install

cargo install --path .

## Usage

codemap init
OPENAI_API_KEY=... codemap update
codemap summary

## Privacy & Cost

- No code is sent unless LLM is enabled.
- Only top candidate files are summarized; unchanged code is not re-sent.
