# llms.json and llms.txt

Create AI guidance helper files at the repo root:

- `llms.json` -> JSON
- `llms.txt` -> markdown text

Purpose: Provide AI tools with a clean, curated map of its most important content.

Help large language models (LLMs) read, understand, and cite a site's documentation or resources without getting bogged down 

File size:  < 40k bytes.

## Repo checkout vs. published site — two link sets, not one

The workspace-root `llms.txt`/`llms.json` use **repo-relative links**
(e.g. `README.md`, `agents/share/overview.md`) — correct for a tool
reading this monorepo as a git checkout, where every linked path
exists on disk.

`*.github.io` publishes a **rendered site**, not a checkout: its own
routes are the only URLs that resolve there. Serving the exact
repo-root text as `*.github.io/llms.txt` would ship links nothing on
that domain answers. The `*.github.io` repo's `static/llms.txt` and
`static/llms.json` are therefore a **separate, website-appropriate
version** — same curated entries, but each `url` rewritten to wherever
that content actually resolves under the site's own domain (e.g. a
`/docs/<name>/` route), never a bare repo-relative path. The two files
are generated from the same map and kept in sync deliberately; they
are not literal copies of each other.
