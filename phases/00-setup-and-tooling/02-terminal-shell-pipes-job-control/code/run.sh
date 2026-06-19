#!/usr/bin/env bash
# Runnable drills for L02: terminal, shell, pipes, job control.
# Each section is a self-contained demonstration you can run line-by-line.
# Usage:  bash run.sh   (runs all demos; output annotated)

set -uo pipefail   # NOTE: not -e — we want to keep going past expected non-zero exits

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "Top shell commands by frequency (from this shell session)"
# Synthetic history (so the demo is deterministic when piped)
printf '1 ls\n2 cd /tmp\n3 ls\n4 git status\n5 ls\n6 git status\n7 vim\n' \
  | awk '{print $2}' | sort | uniq -c | sort -rn | head -5

hr "Three FDs, three streams"
# Use a temporary script to avoid 'EOF' parsing issues in heredocs.
demo=$(mktemp)
cat > "$demo" <<'EOF'
#!/usr/bin/env bash
echo "stdout line"
echo "stderr line" >&2
EOF
chmod +x "$demo"

echo "(1) no redirect — both streams reach the terminal:"
"$demo"

echo "(2) > out.txt 2>&1 — both into one file (read 'send fd 2 wherever fd 1 points'):"
"$demo" > /tmp/out.combined 2>&1
cat /tmp/out.combined

echo "(3) > out.txt 2> err.txt — split:"
"$demo" > /tmp/out.stdout 2> /tmp/out.stderr
echo "  stdout file: $(cat /tmp/out.stdout)"
echo "  stderr file: $(cat /tmp/out.stderr)"

hr "Why order matters: 2>&1 before > redirects to the original stdout (terminal)"
echo "Wrong order: 2>&1 > /tmp/wrong"
"$demo" 2>&1 > /tmp/wrong
echo "  /tmp/wrong contains (stderr leaked to terminal above):"
cat /tmp/wrong

hr "set -o pipefail demo"
echo "Without pipefail:  false | true ; exit=$? (note: 0, hides failure)"
( set +o pipefail; false | true; echo "exit=$?" )
echo "With pipefail:     false | true ; exit=$? (note: 1, surfaces failure)"
( set -o pipefail; false | true; echo "exit=$?" )

hr "Job control sketch (run interactively to see %1)"
cat <<'EOM'
Try this in an interactive shell:

  sleep 30 &        # backgrounded; shell prints [1] PID
  jobs              # list jobs
  fg %1             # foreground job 1
  # Press Ctrl-Z    # SIGTSTP — suspended
  bg %1             # resume in background
  kill %1           # terminate via job id (not PID)
EOM

rm -f "$demo" /tmp/out.combined /tmp/out.stdout /tmp/out.stderr /tmp/wrong
echo
echo "Done."
