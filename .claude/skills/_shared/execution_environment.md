# Execution Environment (shared)

Single source for chimera-code-taste and chimera-sprint-discipline. Edit here
only — both skills point at this file instead of carrying their own copy.

Project Chimera development host: Windows.
Tool invocations use the PowerShell tool (pwsh 7+), NOT Bash.

PowerShell-native idioms to use:
  - File listing: Get-ChildItem path/ -Recurse -Filter *.py
  - Existence check: Test-Path path/
  - Pattern search: Select-String -Path 'path/*.py' -Pattern 'foo' -Recurse
  - File content: Get-Content path/file.md
  - Append to file: "text" | Out-File -Append -Encoding utf8 path/file.md
  - Remove file: Remove-Item path/file.md -Force
  - Conditional: $x ? "yes" : "no" (pwsh 7+ ternary)
  - Null coalescing: $x ?? "default" (pwsh 7+)

Cross-platform commands that work as-is:
  - git log/diff/status/show/add/commit
  - python -m pytest/ruff/mypy
  - cargo test/build
  - npm run check / pnpm check

Do NOT use Unix-only syntax:
  - find (use Get-ChildItem -Recurse)
  - grep (use Select-String)
  - rm (use Remove-Item)
  - cat (use Get-Content — alias exists but not in tool globs)
  - test -f (use Test-Path)
  - echo "x" >> file (use Out-File -Append)
  - [ -d path ] (use Test-Path -PathType Container)
  - wc -l (use (Get-Content file).Count)

Construct correct syntax on first attempt. No POSIX-then-retry pattern.
