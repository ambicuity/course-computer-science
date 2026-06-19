# Git Deep — Internals, Refs, Rebase, Bisect

> Git is a content-addressable filesystem with a porcelain on top. Learn the filesystem and the porcelain becomes obvious.

**Type:** Learn
**Languages:** Shell
**Prerequisites:** Phase 00, Lessons 01–02
**Time:** ~90 minutes

## Learning Objectives

- Describe what a blob, tree, commit, and tag *physically* are in `.git/objects/`.
- Move HEAD around safely with `reset` (soft/mixed/hard), `checkout`, `switch`, `restore`, and explain when each one changes the working tree.
- Use `rebase -i` to reshape history (squash, reorder, fixup), and use `rebase --onto` to transplant a branch.
- Find the commit that broke a test in O(log n) commits with `git bisect`, automated or interactive.

## The Problem

Most developers learn enough git to commit, push, and survive merge conflicts. That's enough until it isn't — until you need to:

- Rewrite the last 12 commits because a co-author added a real `.env` file with production credentials and you have to scrub it from history before pushing.
- Find which of the 600 commits since the last release introduced a regression, when running the test suite by hand on each one would take days.
- Move a branch that was based on `feature-A` to be based on `main` instead, without losing the commits or creating merge cruft.
- Recover work after typing `git reset --hard HEAD~5` and watching five days of commits "disappear."

Each of these is a one-line git command — *if* you have the mental model. Without it, you copy answers from Stack Overflow that nearly work and produce repos with mangled history that no one can read. This lesson builds the model: object database, refs, HEAD, index, working tree. Then `rebase` and `bisect` become consequences, not magic.

## The Concept

### Git is a content-addressable filesystem

Forget commits and branches for a moment. The core of git is a key-value store on disk:

- **Key:** the SHA-1 (or SHA-256 in modern git) hash of the content.
- **Value:** zlib-compressed bytes.

Everything you do with git ultimately reads or writes that store. The store lives in `.git/objects/`. There are four object types:

| Type | What it stores | Analogy |
|------|----------------|---------|
| **blob** | The raw bytes of a single file | A leaf in a filesystem |
| **tree** | A list of (mode, name, blob-or-tree-hash) entries | A directory |
| **commit** | A pointer to a tree (the snapshot), pointer(s) to parent commit(s), author, committer, message | A versioned snapshot |
| **tag** (annotated) | A pointer to any other object, plus metadata and signature | A label with provenance |

```
                       tree (root snapshot)
                       ├── 100644 README.md → blob abc...
                       ├── 100755 build.sh  → blob def...
                       └── 040000 src/      → tree ghi...
                                              ├── 100644 main.c → blob ...
                                              └── 100644 util.c → blob ...
```

A **commit** is a small text file whose first line points to a tree. That's it. There is no "diff" stored anywhere. Diffs are computed on demand by comparing two trees.

### Branches and tags are just refs

A **ref** is a text file containing the SHA of an object. That's literally all a branch is:

```
$ cat .git/refs/heads/main
8c4d2f1e7a93b...
```

Move that file to point at a different SHA, and the branch "moves." `git branch -f`, `git reset`, and `git push --force` all do exactly that.

**HEAD** is the special ref that records "which branch are we on right now?":

```
$ cat .git/HEAD
ref: refs/heads/main
```

If HEAD points directly at a SHA instead of a ref name, you're in "detached HEAD" — your work goes to commits that no branch tracks, so they'll be garbage collected after `gc.reflogExpireUnreachable` (default: 30 days).

### Three trees: HEAD, index, working tree

When you `git status`, you're comparing three layers:

```
   HEAD             index             working tree
 (last commit's    (the staging       (your files
  tree)             area, .git/index)  on disk)
        │                  │                  │
        ▼                  ▼                  ▼
        ────── git diff ─────                  │
                          ────── git diff ────┘
                                  (working tree)
```

| Layer | What changes it |
|-------|-----------------|
| HEAD | `commit`, `reset`, `checkout`, `merge`, `rebase` |
| Index | `git add`, `git reset` (without `--soft`), `git rm --cached`, `git restore --staged` |
| Working tree | `git checkout`, `git restore`, `git reset --hard`, your editor |

Most of git's "I don't understand what happened" moments come from not knowing which of those three a command touched. Once you can name the three, every command makes sense.

### `git reset` in one diagram

```
  --soft               --mixed (default)         --hard
   moves                 moves                    moves
   HEAD                  HEAD                     HEAD
                          +                        +
                       resets                   resets index
                       index                    + resets working tree
                                                (DANGEROUS — work lost)
```

- `git reset --soft  HEAD~3` — move HEAD back 3 commits, keep index and working tree unchanged. Useful for "I want to squash my last three commits into one."
- `git reset --mixed HEAD~3` — also unstage the changes. Useful for "redo the staging from scratch."
- `git reset --hard  HEAD~3` — also delete the changes from the working tree. Use with care.

Lost work after `--hard`? Look at `git reflog` — it records every move of HEAD for the last 90 days. Commits that no branch points at are still recoverable through the reflog.

### `git rebase` rewrites history

`git rebase` takes a series of commits and *replays* them onto a new base, producing new commits with the same patches but different hashes (and possibly different content if there are conflicts).

```
Before:                          After git rebase main on feature:

  A--B--C  (main)                A--B--C  (main)
      \                              \
       D--E--F (feature)              D'--E'--F' (feature)
```

`-i` (interactive) lets you edit, squash, reorder, or drop commits as they're replayed. `--onto` lets you say "replay these commits onto a totally different base":

```sh
# Move just commits E and F to be based on main, dropping D
git rebase --onto main D feature
```

Rule of thumb: **never rebase commits that have been pushed and shared**, because rewriting their hashes will hurt every collaborator.

### `git bisect` is binary search over commits

You have:
- A "bad" commit where a test fails.
- A "good" commit (earlier) where it passed.
- 600 commits in between.

`git bisect` walks log₂(600) ≈ 10 commits, asking you each time "good or bad?" When you give it a script, it does the walk automatically.

```sh
git bisect start
git bisect bad                # current commit is broken
git bisect good v1.2.0        # this old tag works
# git checks out a commit ~halfway between
make test                     # or run any check
git bisect bad                # or `good` — repeat ~log2(N) times
# git prints the first bad commit
git bisect reset
```

For automation:

```sh
git bisect run ./run_test.sh
# script must exit 0 (good), 125 (skip), 1-124 or 126-127 (bad)
```

## Build It

### Step 1: Inspect the object database by hand

```sh
mkdir gitdeep && cd gitdeep
git init -q

echo "hello" > a.txt
git add a.txt
git commit -m "first commit" -q

# What's in .git/objects?
find .git/objects/ -type f
# Three objects: one blob (file content), one tree (root directory), one commit
```

Look at each by hash:

```sh
git cat-file -t <sha>         # type: blob, tree, commit, tag
git cat-file -p <sha>         # pretty-printed content
```

Walk the chain manually:

```sh
HEAD=$(git rev-parse HEAD)
git cat-file -p $HEAD              # commit — shows tree SHA + parents + message
TREE=$(git cat-file -p $HEAD | awk '/^tree/{print $2}')
git cat-file -p $TREE              # tree — shows mode, type, sha, name
BLOB=$(git cat-file -p $TREE | awk '{print $3}')
git cat-file -p $BLOB              # blob — shows file content "hello"
```

You just hand-walked the data structure git uses for every operation.

### Step 2: Confirm "branches are just files"

```sh
git branch experiment
ls .git/refs/heads/
cat .git/refs/heads/experiment       # same SHA as main right now

# Move the branch by editing the file (don't do this in real repos)
git rev-parse HEAD~0 > .git/refs/heads/experiment   # contrived no-op
```

(Use `git branch -f` or `git update-ref` for the real version of this.)

### Step 3: Reset, demystified

```sh
echo "line 1" > b.txt; git add b.txt; git commit -m "add b" -q
echo "line 2" >> b.txt; git add b.txt; git commit -m "update b" -q

git log --oneline                    # see the two commits

git reset --soft HEAD~1              # rewind HEAD, keep index, keep wd
git status                           # changes are staged

git reset --mixed HEAD~1             # try again — also unstage
git status                           # changes are unstaged but still in wd

# WARNING — destructive
git reset --hard HEAD                # back to current HEAD, throw away wd changes
```

If you've lost work to `--hard`, run `git reflog`. Every entry is a recoverable HEAD position.

### Step 4: Interactive rebase to clean up history

```sh
for i in 1 2 3 4; do
  echo "edit $i" >> note.txt
  git add note.txt
  git commit -m "WIP $i" -q
done

git log --oneline -5
# Squash all four WIPs into one tidy commit
git rebase -i HEAD~4
# Editor opens with:
#   pick aaa1111 WIP 1
#   pick bbb2222 WIP 2
#   pick ccc3333 WIP 3
#   pick ddd4444 WIP 4
# Change to:
#   pick aaa1111 WIP 1
#   squash bbb2222 WIP 2
#   squash ccc3333 WIP 3
#   squash ddd4444 WIP 4
# Save and exit; another editor lets you write a single commit message.

git log --oneline -3
# Now one commit instead of four.
```

### Step 5: Bisect a broken test

Set up a synthetic scenario:

```sh
for i in $(seq 1 10); do
  printf "line %d\n" "$i" >> data.txt
  git add data.txt
  git commit -m "add line $i" -q
done

# Pretend commit #7 introduced a bug: grep the file for "line 7"
# We'll write a test that fails when "line 7" is present (i.e., on commit 7+)

cat > test.sh <<'EOF'
#!/usr/bin/env bash
grep -q "line 7" data.txt && exit 1 || exit 0
EOF
chmod +x test.sh

git bisect start
git bisect bad HEAD                                 # last commit fails
git bisect good HEAD~9                              # first commit passes
git bisect run ./test.sh
# git prints: "<sha> is the first bad commit" — the one that added line 7
git bisect reset
```

Bisect ran the test ~4 times on a 9-commit range. In real codebases the savings are dramatic — 600 commits ≈ 10 runs.

## Use It

Real git workflows are layered on the same primitives:

- **Pull requests / merge requests** = "compare two trees at two commits, present the diff, allow merging by fast-forward or with a merge commit."
- **Force-push protection** on `main` = "refuse to update the ref unless the new tip's history contains the current tip" (i.e., a fast-forward).
- **`git filter-repo`** (the modern `filter-branch` replacement) does exactly what you'd do by hand: walk the commit graph, rewrite each commit's tree, emit new commits.

The CI on this repo runs `git log --no-merges` to enumerate changes for the changelog. The agent SDKs in Phase 13 use `git diff --stat` to summarize PRs. Once you can see the object database, every higher-level tool fits in your head.

## Read the Source

- `https://github.com/git/git/blob/master/Documentation/gitrepository-layout.txt` — the official map of `.git/`. Print it; refer to it.
- `https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain` — the "Pro Git" book's internals chapter. Free, ~70 pages, worth every minute.
- `https://github.com/git/git/blob/master/cache.h` — the `index_state` struct. Once you read it, "the index" stops being abstract.

## Ship It

This lesson's artifact is **`outputs/git-rescue.sh`** — a script that, given the path of a repo, prints a "you broke something" rescue panel: current HEAD, reflog tail, dangling commits, untracked but ignored files, and the most recent stash. Print, paste in chat when someone says "git ate my work."

## Exercises

1. **Easy.** Create a tiny repo with three commits. Without using `git log`, walk from HEAD to the root by hand using only `git cat-file -p`. Sketch the commit chain.
2. **Medium.** Set up two branches that diverged 5 commits each, then use `git rebase --onto` to move just commits 3–5 of one branch onto the tip of the other. Verify with `git log --graph --oneline --all`.
3. **Hard.** Write a `git bisect run` script in 30 lines or fewer that bisects a "performance regression" — i.e., the script measures something (build time, benchmark output) and marks a commit "bad" when the metric exceeds a threshold. Hint: use `git bisect skip` for commits where the metric can't be measured.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Branch | "A line of development" | A movable text file under `.git/refs/heads/` containing the SHA of one commit |
| HEAD | "Current commit" | The pointer to the active branch (or the active SHA, when detached) |
| Index / staging area | "What's going into the next commit" | A binary file at `.git/index` that holds the snapshot to commit |
| Detached HEAD | "Lost mode" | HEAD points directly at a SHA rather than a branch; new commits aren't tracked by any ref |
| Rebase | "Replay onto a new base" | Cherry-pick each commit in order, rewriting hashes |

## Further Reading

- *Pro Git* (Scott Chacon & Ben Straub) — free online, the canonical reference.
- [Think Like (a) Git](http://think-like-a-git.net/) — an essay on graph thinking for git.
- [git from the inside out](https://maryrosecook.com/blog/post/git-from-the-inside-out) — Mary Rose Cook's walk through the same object-database mental model.
