# PowerShell script for publishing objectiveai-ensemble
$ErrorActionPreference = "Stop"

# Default options
$DryRun = $true
$SkipTests = $false
$SkipGitTag = $false
$CratesOnly = $false
$NpmOnly = $false

# Parse command line arguments
function Show-Help {
    Write-Host @"
Usage: .\publish.ps1 [OPTIONS]

Package and publish objectiveai-ensemble to crates.io and npm.

OPTIONS:
    -NoDryRun           Actually perform publishing (default: dry-run only)
    -CratesOnly         Only publish to crates.io, skip npm
    -NpmOnly            Only publish to npm (assumes crates already published)
    -SkipTests          Skip running tests before publishing
    -SkipGitTag         Skip creating and pushing git tag
    -Help               Show this help message

EXAMPLES:
    .\publish.ps1                           # Dry-run (test without publishing)
    .\publish.ps1 -NoDryRun                 # Actually publish everything
    .\publish.ps1 -NoDryRun -CratesOnly     # Publish only to crates.io
    .\publish.ps1 -NoDryRun -NpmOnly        # Publish only to npm
    .\publish.ps1 -NoDryRun -SkipTests      # Publish without running tests

PREREQUISITES:
    - cargo (with 'cargo login' completed)
    - wasm-pack (install: cargo install wasm-pack)
    - npm (with 'npm login' completed)
    - node (for package.json manipulation)
    - git (for tagging)

"@
    exit 0
}

foreach ($arg in $args) {
    switch ($arg) {
        "-NoDryRun" { $DryRun = $false }
        "-SkipTests" { $SkipTests = $true }
        "-SkipGitTag" { $SkipGitTag = $true }
        "-CratesOnly" { $CratesOnly = $true }
        "-NpmOnly" { $NpmOnly = $true }
        "-Help" { Show-Help }
        default {
            Write-Host "Unknown option: $arg" -ForegroundColor Red
            Write-Host "Use -Help for usage information"
            exit 1
        }
    }
}

# Validation
if ($CratesOnly -and $NpmOnly) {
    Write-Host "Error: Cannot use both -CratesOnly and -NpmOnly" -ForegroundColor Red
    exit 1
}

# Helper functions
function Log-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Log-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Log-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Test-Command {
    param(
        [string]$Command,
        [string]$InstallMessage
    )

    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        Log-Error "$Command is not installed. Please install it first."
        if ($InstallMessage) {
            Write-Host "  Install with: $InstallMessage"
        }
        exit 1
    }
}

function Get-Version {
    $cargoToml = Get-Content "objectiveai-ensemble\Cargo.toml"
    $versionLine = $cargoToml | Select-String '^version' | Select-Object -First 1
    if ($versionLine -match 'version\s*=\s*"([^"]+)"') {
        return $matches[1]
    }
    throw "Could not find version in Cargo.toml"
}

# Print header
Write-Host "================================================"
Write-Host "  ObjectiveAI Ensemble - Build & Publish"
Write-Host "================================================"
Write-Host ""

if ($DryRun) {
    Log-Warn "DRY-RUN MODE: No actual publishing will occur"
    Log-Warn "Use -NoDryRun to actually publish"
    Write-Host ""
}

# Get version
$Version = Get-Version
Log-Info "Version: $Version"
Write-Host ""

# Step 1: Check prerequisites
Log-Info "Step 1: Checking prerequisites..."

if (-not $NpmOnly) {
    Test-Command "cargo" "Visit https://rustup.rs"
}

if (-not $CratesOnly) {
    Test-Command "wasm-pack" "cargo install wasm-pack"
    Test-Command "npm" "Visit https://nodejs.org"
    Test-Command "node" "Visit https://nodejs.org"
}

Test-Command "git" "Visit https://git-scm.com"

Log-Info "All required tools are installed"
Write-Host ""

# Step 2: Check authentication
Log-Info "Step 2: Checking authentication..."

if (-not $NpmOnly) {
    # Check cargo authentication
    $credentialsPath1 = Join-Path $env:USERPROFILE ".cargo\credentials.toml"
    $credentialsPath2 = Join-Path $env:USERPROFILE ".cargo\credentials"

    if (-not (Test-Path $credentialsPath1) -and -not (Test-Path $credentialsPath2)) {
        Log-Error "Cargo credentials not found. Run 'cargo login' first."
        exit 1
    }
    Log-Info "Cargo: Authenticated (credentials found)"
}

if (-not $CratesOnly) {
    # Check npm authentication
    try {
        $npmUser = npm whoami 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Not authenticated"
        }
        Log-Info "npm: Authenticated as $npmUser"
    }
    catch {
        Log-Error "npm: Not authenticated. Run 'npm login' first."
        exit 1
    }
}

Write-Host ""

# Step 3: Run tests
if (-not $SkipTests -and -not $NpmOnly) {
    Log-Info "Step 3: Running tests..."
    if ($DryRun) {
        Log-Warn "[DRY-RUN] Would run: cargo test --workspace"
    }
    else {
        cargo test --workspace
        if ($LASTEXITCODE -ne 0) {
            Log-Error "Tests failed"
            exit 1
        }
        Log-Info "All tests passed"
    }
    Write-Host ""
}
elseif (-not $NpmOnly) {
    Log-Warn "Step 3: Skipping tests (-SkipTests)"
    Write-Host ""
}

# Step 4: Publish to crates.io
if (-not $NpmOnly) {
    Log-Info "Step 4: Publishing to crates.io..."

    # Publish objectiveai-ensemble
    Log-Info "Publishing objectiveai-ensemble..."
    if ($DryRun) {
        Log-Warn "[DRY-RUN] Would run: cargo publish -p objectiveai-ensemble"
    }
    else {
        $output = cargo publish -p objectiveai-ensemble 2>&1 | Out-String
        Write-Host $output
        if ($output -match "already uploaded") {
            Log-Warn "objectiveai-ensemble v$Version already published, skipping"
        }
        elseif ($LASTEXITCODE -ne 0) {
            Log-Error "Failed to publish objectiveai-ensemble"
            exit 1
        }
        else {
            Log-Info "objectiveai-ensemble published successfully"
        }
    }

    # Wait for crates.io to index (configurable via CRATES_IO_WAIT_SECONDS)
    $WaitTime = if ($env:CRATES_IO_WAIT_SECONDS) { [int]$env:CRATES_IO_WAIT_SECONDS } else { 30 }
    Log-Info "Waiting $WaitTime seconds for crates.io to index..."
    if (-not $DryRun) {
        Start-Sleep -Seconds $WaitTime
    }

    # Publish objectiveai-ensemble-js
    Log-Info "Publishing objectiveai-ensemble-js..."
    if ($DryRun) {
        Log-Warn "[DRY-RUN] Would run: cargo publish -p objectiveai-ensemble-js"
    }
    else {
        $output = cargo publish -p objectiveai-ensemble-js 2>&1 | Out-String
        Write-Host $output
        if ($output -match "already uploaded") {
            Log-Warn "objectiveai-ensemble-js v$Version already published, skipping"
        }
        elseif ($LASTEXITCODE -ne 0) {
            Log-Error "Failed to publish objectiveai-ensemble-js"
            exit 1
        }
        else {
            Log-Info "objectiveai-ensemble-js published successfully"
        }
    }

    Log-Info "All crates published to crates.io"
    Write-Host ""
}

# Exit if crates-only
if ($CratesOnly) {
    Log-Info "Crates-only mode: Skipping npm publish"
    Write-Host ""
    Log-Info "Done!"
    exit 0
}

# Step 5: Build WASM package
Log-Info "Step 5: Building WASM package with wasm-pack..."
if ($DryRun) {
    Log-Warn "[DRY-RUN] Would run: wasm-pack build --target web objectiveai-ensemble-js"
}
else {
    wasm-pack build --target web objectiveai-ensemble-js
    if ($LASTEXITCODE -ne 0) {
        Log-Error "Failed to build WASM package"
        exit 1
    }
    Log-Info "WASM package built successfully"
}
Write-Host ""

# Step 6: Modify package.json
Log-Info "Step 6: Modifying package.json..."
$PkgDir = "objectiveai-ensemble-js\pkg"
$PkgJson = "$PkgDir\package.json"

if ($DryRun) {
    Log-Warn "[DRY-RUN] Would modify: $PkgJson"
    Log-Warn "[DRY-RUN] Would add: description, repository, homepage, keywords, author"
}
else {
    # Check if package.json exists
    if (-not (Test-Path $PkgJson)) {
        Log-Error "package.json not found at $PkgJson. wasm-pack build may have failed."
        exit 1
    }

    # Use Node.js to safely modify the package.json
    $nodeScript = @'
const fs = require('fs');
const path = require('path');

const pkgPath = path.join('objectiveai-ensemble-js', 'pkg', 'package.json');

// Check if file exists
if (!fs.existsSync(pkgPath)) {
    console.error('Error: package.json not found at', pkgPath);
    process.exit(1);
}

let pkg;
try {
    const content = fs.readFileSync(pkgPath, 'utf8');
    pkg = JSON.parse(content);
} catch (error) {
    console.error('Error: Failed to parse package.json:', error.message);
    process.exit(1);
}

// Ensure name is correct (wasm-pack should set this from Cargo.toml metadata)
pkg.name = '@objectiveai/ensemble';

// Add/update metadata
pkg.description = 'JavaScript WebAssembly bindings for objectiveai-ensemble';
pkg.repository = {
    type: 'git',
    url: 'https://github.com/ObjectiveAI/objectiveai-ensemble'
};
pkg.homepage = 'https://objective-ai.io';
pkg.keywords = ['llm', 'ai', 'ensemble', 'wasm', 'javascript', 'webassembly'];
pkg.author = 'ObjectiveAI <admin@objective-ai.io>';
pkg.license = 'MIT';

try {
    fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
    console.log('package.json updated successfully');
} catch (error) {
    console.error('Error: Failed to write package.json:', error.message);
    process.exit(1);
}
'@

    $nodeScript | node
    if ($LASTEXITCODE -ne 0) {
        Log-Error "Failed to update package.json"
        exit 1
    }
    Log-Info "package.json updated successfully"
}
Write-Host ""

# Step 7: Copy LICENSE
Log-Info "Step 7: Copying LICENSE to pkg directory..."
if ($DryRun) {
    Log-Warn "[DRY-RUN] Would copy: LICENSE -> $PkgDir\LICENSE"
}
else {
    if (-not (Test-Path "LICENSE")) {
        Log-Error "LICENSE file not found in repository root"
        exit 1
    }
    if (-not (Test-Path $PkgDir)) {
        Log-Error "Package directory $PkgDir not found"
        exit 1
    }
    Copy-Item "LICENSE" "$PkgDir\LICENSE"
    Log-Info "LICENSE copied successfully"
}
Write-Host ""

# Step 8: Publish to npm
Log-Info "Step 8: Publishing to npm..."
if ($DryRun) {
    Log-Warn "[DRY-RUN] Would run: npm publish --access public (from $PkgDir)"
}
else {
    Push-Location $PkgDir
    try {
        npm publish --access public
        if ($LASTEXITCODE -ne 0) {
            throw "npm publish failed"
        }
        Log-Info "@objectiveai/ensemble published to npm successfully"
    }
    finally {
        Pop-Location
    }
}
Write-Host ""

# Step 9: Create git tag
if (-not $SkipGitTag) {
    Log-Info "Step 9: Creating git tag..."
    $Tag = "v$Version"

    if ($DryRun) {
        Log-Warn "[DRY-RUN] Would create tag: $Tag"
        Log-Warn "[DRY-RUN] Would run: git tag $Tag && git push origin $Tag"
    }
    else {
        # Check if tag already exists
        $tagExists = git rev-parse $Tag 2>$null
        if ($LASTEXITCODE -eq 0) {
            Log-Warn "Tag $Tag already exists, skipping tag creation"
        }
        else {
            git tag $Tag
            if ($LASTEXITCODE -ne 0) {
                Log-Error "Failed to create git tag $Tag"
                exit 1
            }
            git push origin $Tag
            if ($LASTEXITCODE -ne 0) {
                Log-Error "Failed to push git tag $Tag"
                exit 1
            }
            Log-Info "Git tag $Tag created and pushed"
        }
    }
    Write-Host ""
}
else {
    Log-Warn "Step 9: Skipping git tag creation (-SkipGitTag)"
    Write-Host ""
}

# Success!
Write-Host "================================================"
if ($DryRun) {
    Log-Info "DRY-RUN COMPLETE"
    Write-Host ""
    Log-Info "To actually publish, run:"
    Write-Host "  .\publish.ps1 -NoDryRun"
}
else {
    Log-Info "PUBLISH COMPLETE!"
    Write-Host ""
    Log-Info "Published:"
    if (-not $NpmOnly) {
        Write-Host "  - objectiveai-ensemble v$Version to crates.io"
        Write-Host "  - objectiveai-ensemble-js v$Version to crates.io"
    }
    if (-not $CratesOnly) {
        Write-Host "  - @objectiveai/ensemble v$Version to npm"
    }
}
Write-Host "================================================"
