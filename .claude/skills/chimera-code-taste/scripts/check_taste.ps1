#!/usr/bin/env pwsh
# Quick taste verification: ruff + mypy + pytest for given file(s).
# Usage: ./scripts/check_taste.ps1 src/oligo/core/agent.py [more_files...]
#
# Exit-code contract: this script is the commit gate for batch_execution.
# Each tool's $LASTEXITCODE is checked MANUALLY so the gate works on pwsh < 7.4,
# which does not honor $PSNativeCommandUseErrorActionPreference for native
# commands. Any non-zero exit aborts immediately with that code. The green
# "check complete" banner prints ONLY when every tool exited 0. A source file
# with no matching test is reported as unverified (yellow), not silently passed.

param(
    [Parameter(Mandatory, Position=0, ValueFromRemainingArguments)]
    [string[]]$Files
)

$ErrorActionPreference = 'Stop'  # governs cmdlets (Test-Path etc.); native exit
                                 # codes are checked manually below.

Write-Host "`n=== ruff ===" -ForegroundColor Cyan
python -m ruff check @Files
if ($LASTEXITCODE -ne 0) {
    Write-Host "`n[FAIL] ruff exited $LASTEXITCODE — gate blocked." -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "`n=== mypy ===" -ForegroundColor Cyan
python -m mypy @Files
if ($LASTEXITCODE -ne 0) {
    Write-Host "`n[FAIL] mypy exited $LASTEXITCODE — gate blocked." -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "`n=== pytest (impacted) ===" -ForegroundColor Cyan
foreach ($path in $Files) {
    $base = [System.IO.Path]::GetFileNameWithoutExtension($path)
    $testPath = "tests/oligo/test_$base.py"
    if (Test-Path $testPath) {
        python -m pytest $testPath -x -v
        if ($LASTEXITCODE -ne 0) {
            Write-Host "`n[FAIL] pytest $testPath exited $LASTEXITCODE — gate blocked." -ForegroundColor Red
            exit $LASTEXITCODE
        }
    } else {
        Write-Host "[WARN] No test for $base — skipped (unverified): $testPath" -ForegroundColor Yellow
    }
}

Write-Host "`n=== check complete ===" -ForegroundColor Green
