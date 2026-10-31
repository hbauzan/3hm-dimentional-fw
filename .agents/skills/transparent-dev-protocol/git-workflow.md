# GIT AND VERSION CONTROL WORKFLOW (Gitstuff)

Follow these rules for committing code, running hooks, and maintaining version safety.

---

## 1. GIT METADATA BLOCK

Upon successful completion of a logical task, always append a dedicated Git Metadata block at the absolute end of your response using the following format:

```yaml
Branch Name: <type>/<short-descriptive-name>  # e.g., feat/simd-cosine-kernel or fix/clause-utf8-boundary
Commit Message: <type>(<scope>): <short description in present tense> # e.g., feat(kernel): add avx512 cosine accumulator
```

---

## 2. GIT HOOK CONVENTIONS & STACK HYGIENE

To keep code quality, formatting, and security consistent before any commit is finalized, wire git hooks via the project's native tooling or a portable framework (e.g. `.pre-commit-config.yaml` at the workspace root).

A ready-to-use base hygiene config ships with this protocol at [`templates/.pre-commit-config.yaml`](./templates/.pre-commit-config.yaml) — copy it to the workspace root and adapt hooks to the project's canonical toolchain.

### 2.1. Recommended Hook Setup
Conventions (swappable per app/stack, but stay consistent within a repo):
- **Format + Lint**: Run the canonical linter and formatter on staged files (e.g., `cargo fmt --check && cargo clippy` in Rust; `ruff format && ruff check` in Python; `golangci-lint` in Go; `biome` / `eslint` in TypeScript).
- **Type / Borrow Check**: Fast compiler check or static analysis in CI or pre-commit stage.
- **Secret Scan**: A secret-detection hook (e.g. `gitleaks` or `detect-secrets`) to enforce the "no keys in git" rule from [SKILL.md](./SKILL.md) §3.2.
- **Hygiene**: trailing-whitespace, end-of-file-fixer, check-merge-conflict, and a check that `.env` is never staged.

### 2.2. Installation & Smoke Testing
- Install the git hook once per clone using the project's canonical hook installer (e.g., `pre-commit install` or native git hooks).
- Always smoke-test locally before pushing or resolving a task (e.g., `pre-commit run --all-files` or canonical check script).

---

## 3. GIT DELIVERY & AUTOMATION POLICY

The agent **owns the git lifecycle** — branch, stage, commit, and (after approval) push and integrate — gated by a single mandatory human checkpoint. Nothing in this section authorizes a push or a merge before that checkpoint.

### 3.1. The Approval Gate (mandatory)
- The agent may **freely** create branches, stage files, and commit **locally** at any point while working.
- The agent must **NOT `git push` and must NOT merge to the base branch** until the user has verified the change and given an **explicit go-ahead** (e.g. "ok", "dale", "andá", "mergealo").
- Reporting "ready to test" and then **waiting** is mandatory. Silence, a thumbs-up on something unrelated, or the absence of objection is **not** approval.

### 3.2. Delivery Sequence (run only after approval)
Default sequence once the user approves:
1. `git checkout -b <type>/<short-name>` — if not already on a dedicated task branch.
2. Stage **only files relevant to the task**. Leave unrelated untracked/modified files alone; if scope is unclear, ask (see §3.3).
3. `git commit` using the metadata format from §1. **AUTORÍA EXCLUSIVA (INVARIANTE INNEGOCIABLE)**: El único autor, titular y colaborador permitido en este y cualquier commit es exclusivamente **hbauzan** (`Héctor Andrés Bauzán Saavedra` / `hbauzan@gmail.com`). Está **TERMINANTEMENTE PROHIBIDO** añadir trailers `Co-authored-by:`, bots o agentes de IA (`cursoragent`, `Cursor`, `claude`, `copilot`, `gemini`, etc.) o cualquier otro colaborador adicional. NUNCA, BAJO NINGUNA CIRCUNSTANCIA, agregar otro colaborador más que `hbauzan`.
4. `git push -u origin <branch>`.
5. Integrate. Pick the path **before** touching the base:
   - **PR path** when the user asked for a PR, the base branch is protected, or the repo documents a PR workflow (CONTRIBUTING, pull-request template, branch protection). Run `gh pr create` and stop. Do not merge and do not `git push origin <base>`. Merging the PR needs its own explicit OK, unless the approval already included the merge.
   - **Direct path** otherwise — this remains the default. `git checkout <base>` → `git merge --no-ff <branch>` → `git push origin <base>`. (`<base>` is usually `main`.)
6. *(Optional, ask first)* delete the merged branch locally and on the remote.

`gh` on PATH is how you **run** the PR path. When `gh` is authenticated and no positive PR signal already dictates the choice, **propose** both paths (PR via `gh pr create` vs direct merge) in the approval report (§3.1) so the user's "OK" can choose. A plain "OK" without preference defaults to the direct path. Do not probe a protected base with a push when a PR signal is already positive.

When `gh` is authenticated, you can check before choosing:

```bash
gh api "repos/$(gh repo view --json nameWithOwner -q .nameWithOwner)/branches/<base>/protection"
```

A protection payload selects the PR path. A 404, a 403, or a failed `gh repo view` confirms nothing: keep the direct path unless a positive PR signal already exists. If `gh` is missing and the PR path is required, stop and ask. If a direct push to the base is rejected, switch to the PR path if `gh` works; otherwise stop and ask the user (§3.3) — do not retry the push or force-push.

### 3.3. Stop-and-Ask Conditions ("when it gets complicated")
**Pause and ask the user** before continuing if any of these arise during delivery:
- Merge conflicts, or the base branch has diverged / moved since branching.
- A pre-commit hook, type check, test, or CI check **fails**.
- A push to the base is **rejected** (protected branch or permissions). Switch to the PR path if `gh` works; otherwise stop and ask. Do not force-push.
- A **force-push** (`--force` / `--force-with-lease`) would be required.
- Commit **scope is ambiguous** (unrelated changes staged, or unrelated untracked files present that might belong in the commit).
- The remote, credentials, or target branch are **not what was expected**.

### 3.4. Destructive Commands (always require explicit confirmation)
These are **not** part of the normal flow and risk irreversible data loss. Never run them autonomously — propose the command and get an explicit "yes" first:
- `git reset --hard` (prefer a soft reset or `git restore <file>`).
- `git clean -f` / `git clean -fd`.
- `git branch -D`.
- `git checkout .` / `git restore .` (reverting the entire working directory).
- Any history rewrite on an already-pushed branch (`rebase`, `commit --amend` after push, force-push).

### 3.5. IDE & Agentic Hooks Integration
Optionally register a `PreToolUse` matcher hook (e.g., `.claude/hooks/block-dangerous-git.sh`) that intercepts **only the §3.4 destructive commands**. `git push` and `git merge` must **not** be blocked by hooks — they are governed by the approval gate (§3.1).
