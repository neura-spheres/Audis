<#
.SYNOPSIS
    Sets up the C++ build environment needed to compile the local Whisper engine.

.DESCRIPTION
    whisper.cpp is C++, so building Audis needs more than Rust. This script finds
    the pieces and exports them into the current session. Dot-source it:

        . ./scripts/env.ps1

    None of this reaches end users. The shipped Audis.exe has no such
    dependencies; this is build-time only. See docs/adr/ADR-011.
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Find-VsDevShell {
    # VS 2022 is required: 2019's bundled cmake predates the "Visual Studio 17
    # 2022" generator and cannot configure whisper.cpp.
    $candidates = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Community",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\BuildTools",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community"
    )
    foreach ($root in $candidates) {
        $vcvars = Join-Path $root 'VC\Auxiliary\Build\vcvars64.bat'
        if (Test-Path $vcvars) { return $vcvars }
    }
    return $null
}

function Find-Ninja {
    # An explicit search rather than trusting PATH. Chromium's depot_tools ships
    # a `ninja` shim that CMake cannot run ("inappropriate file type or
    # format"), and it commonly sits ahead of the real one.
    $candidates = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja",
        "$env:LOCALAPPDATA\Programs\Python\Python312\Scripts"
    )
    foreach ($dir in $candidates) {
        if (Test-Path (Join-Path $dir 'ninja.exe')) { return $dir }
    }
    return $null
}

function Find-LibClang {
    $candidates = @(
        "${env:ProgramFiles}\LLVM\bin",
        "$env:LOCALAPPDATA\Programs\Python\Python312\Lib\site-packages\clang\native",
        "$env:LOCALAPPDATA\Programs\Python\Python311\Lib\site-packages\clang\native"
    )
    foreach ($dir in $candidates) {
        if (Test-Path (Join-Path $dir 'libclang.dll')) { return $dir }
    }
    return $null
}

# The MSVC environment. Imported by running vcvars64 and copying back what it set.
$vcvars = Find-VsDevShell
if (-not $vcvars) {
    Write-Warning 'VS 2022 Build Tools not found. The local Whisper engine will not build.'
    Write-Warning 'Install with: winget install Microsoft.VisualStudio.2022.BuildTools'
}
else {
    cmd /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            Set-Item -Path "Env:$($matches[1])" -Value $matches[2] -ErrorAction SilentlyContinue
        }
    }
    Write-Host "MSVC environment loaded from $vcvars" -ForegroundColor Green
}

# bindgen needs libclang to parse whisper.cpp's headers.
$libclang = Find-LibClang
if (-not $libclang) {
    Write-Warning 'libclang not found. The local Whisper engine will not build.'
    Write-Warning 'Install with: winget install LLVM.LLVM   (or: pip install libclang)'
}
else {
    $env:LIBCLANG_PATH = $libclang
    Write-Host "LIBCLANG_PATH = $libclang" -ForegroundColor Green
}

# Ninja rather than a Visual Studio generator: the VS generator wants a
# "generator instance" that conflicts with how cmake-rs invokes it.
$env:CMAKE_GENERATOR = 'Ninja'
Write-Host 'CMAKE_GENERATOR = Ninja' -ForegroundColor Green

$ninja = Find-Ninja
if (-not $ninja) {
    Write-Warning 'ninja not found. The local Whisper engine will not build.'
}
else {
    # Prepended, not appended: the point is to win against whatever `ninja`
    # already sits on PATH.
    $env:PATH = "$ninja;$env:PATH"
    # Named explicitly as well, so CMake cannot resolve `ninja` from PATH and
    # find the wrong one anyway.
    $env:CMAKE_MAKE_PROGRAM = (Join-Path $ninja 'ninja.exe')
    Write-Host "ninja = $ninja" -ForegroundColor Green
}

# MSVC still enforces MAX_PATH (260). CMake's try-compile directories nest
# deeply under the target dir, so a long checkout fails with a message that
# never mentions path length:
#   fatal error C1083: Cannot open compiler generated file: '': Invalid argument
$repoRoot = Split-Path -Parent $PSScriptRoot
if ($repoRoot.Length -gt 40) {
    Write-Warning "This repo is at a long path ($($repoRoot.Length) chars): $repoRoot"
    Write-Warning 'whisper.cpp may fail with a misleading C1083 error. Move the repo nearer the drive root.'
}
