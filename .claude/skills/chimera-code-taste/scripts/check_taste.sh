#!/usr/bin/env bash
# Quick taste verification: ruff + mypy + pytest for given file(s).
# Usage: ./scripts/check_taste.sh src/oligo/core/agent.py [more_files...]

set -e

if [ $# -eq 0 ]; then
  echo "Usage: $0 <file_or_dir> [more...]"
  exit 1
fi

echo "═══ ruff ═══"
ruff check "$@"

echo
echo "═══ mypy ═══"
mypy "$@"

echo
echo "═══ pytest (impacted) ═══"
# Heuristic: find tests adjacent to modified files
for path in "$@"; do
  base=$(basename "$path" .py)
  test_path="tests/oligo/test_${base}.py"
  if [ -f "$test_path" ]; then
    pytest "$test_path" -x -v
  fi
done

echo
echo "═══ check complete ═══"