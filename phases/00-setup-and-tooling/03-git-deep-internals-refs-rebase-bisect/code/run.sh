#!/usr/bin/env bash
# Hands-on tour of git internals + a working `git bisect run` demo.
# Creates a throwaway repo in a temp dir; cleans up at the end.

set -uo pipefail

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "Init a throwaway repo at $WORK"
git init -q -b main
git config user.email "lesson@example.com"
git config user.name "Lesson"

hr "One commit, three objects in .git/objects/"
echo "hello" > a.txt
git add a.txt
git commit -m "first commit" -q

echo "Object files:"
find .git/objects -type f | sed 's/^/  /'
echo
echo "Types:"
find .git/objects -type f | while read -r f; do
  sha="$(basename "$(dirname "$f")")$(basename "$f")"
  printf "  %s  %s\n" "$sha" "$(git cat-file -t "$sha")"
done

hr "Walk commit -> tree -> blob"
H=$(git rev-parse HEAD)
echo "HEAD = $H"
echo
echo "commit object:"
git cat-file -p "$H" | sed 's/^/  /'
echo
T=$(git cat-file -p "$H" | awk '/^tree/{print $2}')
echo "tree $T:"
git cat-file -p "$T" | sed 's/^/  /'
echo
B=$(git cat-file -p "$T" | awk '{print $3}')
echo "blob $B (file content):"
git cat-file -p "$B" | sed 's/^/  /'

hr "Branches are files: cat .git/refs/heads/main"
cat .git/refs/heads/main | sed 's/^/  /'

hr "Build a 10-commit history that introduces 'line 7' on commit 7"
for i in 1 2 3 4 5 6 7 8 9 10; do
  printf "line %d\n" "$i" >> data.txt
  git add data.txt
  git commit -m "add line $i" -q
done
git log --oneline | head | sed 's/^/  /'

hr "Bisect to find which commit introduced 'line 7'"
# Test exits 1 (bad) when "line 7" is present, 0 (good) otherwise.
cat > test.sh <<'EOF'
#!/usr/bin/env bash
grep -q "line 7" data.txt && exit 1 || exit 0
EOF
chmod +x test.sh

git bisect start >/dev/null
git bisect bad HEAD >/dev/null
git bisect good HEAD~9 >/dev/null
echo "Bisect run:"
git bisect run ./test.sh 2>&1 | tail -10 | sed 's/^/  /'
git bisect reset >/dev/null

hr "Reflog records every HEAD move (recover work after --hard)"
git log --oneline | head -3 | sed 's/^/  /'
git reset --hard HEAD~3
echo "After hard reset:"
git log --oneline | head -3 | sed 's/^/  /'
echo "Reflog still knows where HEAD was:"
git reflog | head -5 | sed 's/^/  /'

echo
echo "Done. Repo cleaned up: $WORK"
