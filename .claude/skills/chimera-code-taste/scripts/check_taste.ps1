#!/usr/bin/env pwsh
# Quick taste verification: ruff + mypy + pytest for given file(s).
# Usage: ./scripts/check_taste.ps1 src/oligo/core/agent.py [more_files...]

param(
    [Parameter(Mandatory, Position=0, ValueFromRemainingArguments)]
    [string[]]$Files
)

$ErrorActionPreference = 'Stop'

Write-Host "`n=== ruff ===" -ForegroundColor Cyan
python -m ruff check @Files

Write-Host "`n=== mypy ===" -ForegroundColor Cyan
python -m mypy @Files

Write-Host "`n=== pytest (impacted) ===" -ForegroundColor Cyan
foreach ($path in $Files) {
    $base = [System.IO.Path]::GetFileNameWithoutExtension($path)
    $testPath = "tests/oligo/test_$base.py"
    if (Test-Path $testPath) {
        python -m pytest $testPath -x -v
    }
}

Write-Host "`n=== check complete ===" -ForegroundColor Green
