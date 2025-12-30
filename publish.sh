#!/usr/bin/env bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default options
DRY_RUN=true
SKIP_TESTS=false
SKIP_GIT_TAG=false
CRATES_ONLY=false
NPM_ONLY=false

# Parse command line arguments
show_help() {
    cat << EOF
Usage: ./publish.sh [OPTIONS]

Package and publish objectiveai-ensemble to crates.io and npm.

OPTIONS:
    --no-dry-run        Actually perform publishing (default: dry-run only)
    --crates-only       Only publish to crates.io, skip npm
    --npm-only          Only publish to npm (assumes crates already published)
    --skip-tests        Skip running tests before publishing
    --skip-git-tag      Skip creating and pushing git tag
    --help              Show this help message

EXAMPLES:
    ./publish.sh                              # Dry-run (test without publishing)
    ./publish.sh --no-dry-run                 # Actually publish everything
    ./publish.sh --no-dry-run --crates-only   # Publish only to crates.io
    ./publish.sh --no-dry-run --npm-only      # Publish only to npm
    ./publish.sh --no-dry-run --skip-tests    # Publish without running tests

PREREQUISITES:
    - cargo (with 'cargo login' completed)
    - wasm-pack (install: cargo install wasm-pack)
    - npm (with 'npm login' completed)
    - node (for package.json manipulation)
    - git (for tagging)

EOF
    exit 0
}

for arg in "$@"; do
    case $arg in
        --no-dry-run)
            DRY_RUN=false
            ;;
        --skip-tests)
            SKIP_TESTS=true
            ;;
        --skip-git-tag)
            SKIP_GIT_TAG=true
            ;;
        --crates-only)
            CRATES_ONLY=true
            ;;
        --npm-only)
            NPM_ONLY=true
            ;;
        --help)
            show_help
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Validation
if [ "$CRATES_ONLY" = true ] && [ "$NPM_ONLY" = true ]; then
    echo -e "${RED}Error: Cannot use both --crates-only and --npm-only${NC}"
    exit 1
fi

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "$1 is not installed. Please install it first."
        if [ -n "$2" ]; then
            echo "  Install with: $2"
        fi
        exit 1
    fi
}

# Get version from Cargo.toml
get_version() {
    grep '^version' objectiveai-ensemble/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

# Print header
echo "================================================"
echo "  ObjectiveAI Ensemble - Build & Publish"
echo "================================================"
echo ""

if [ "$DRY_RUN" = true ]; then
    log_warn "DRY-RUN MODE: No actual publishing will occur"
    log_warn "Use --no-dry-run to actually publish"
    echo ""
fi

# Get version
VERSION=$(get_version)
log_info "Version: $VERSION"
echo ""

# Step 1: Check prerequisites
log_info "Step 1: Checking prerequisites..."

if [ "$NPM_ONLY" = false ]; then
    check_command "cargo" "Visit https://rustup.rs"
fi

if [ "$CRATES_ONLY" = false ]; then
    check_command "wasm-pack" "cargo install wasm-pack"
    check_command "npm" "Visit https://nodejs.org"
    check_command "node" "Visit https://nodejs.org"
fi

check_command "git" "Visit https://git-scm.com"

log_info "All required tools are installed"
echo ""

# Step 2: Check authentication
log_info "Step 2: Checking authentication..."

if [ "$NPM_ONLY" = false ]; then
    # Check cargo authentication by trying to read the token
    if [ ! -f "$HOME/.cargo/credentials.toml" ] && [ ! -f "$HOME/.cargo/credentials" ]; then
        log_error "Cargo credentials not found. Run 'cargo login' first."
        exit 1
    fi
    log_info "Cargo: Authenticated (credentials found)"
fi

if [ "$CRATES_ONLY" = false ]; then
    # Check npm authentication
    if ! npm whoami &> /dev/null; then
        log_error "npm: Not authenticated. Run 'npm login' first."
        exit 1
    fi
    NPM_USER=$(npm whoami)
    log_info "npm: Authenticated as $NPM_USER"
fi

echo ""

# Step 3: Run tests
if [ "$SKIP_TESTS" = false ] && [ "$NPM_ONLY" = false ]; then
    log_info "Step 3: Running tests..."
    if [ "$DRY_RUN" = true ]; then
        log_warn "[DRY-RUN] Would run: cargo test --workspace"
    else
        cargo test --workspace
        log_info "All tests passed"
    fi
    echo ""
else
    if [ "$NPM_ONLY" = false ]; then
        log_warn "Step 3: Skipping tests (--skip-tests)"
        echo ""
    fi
fi

# Step 4: Publish to crates.io
if [ "$NPM_ONLY" = false ]; then
    log_info "Step 4: Publishing to crates.io..."

    # Publish objectiveai-ensemble
    log_info "Publishing objectiveai-ensemble..."
    if [ "$DRY_RUN" = true ]; then
        log_warn "[DRY-RUN] Would run: cargo publish -p objectiveai-ensemble"
    else
        if ! cargo publish -p objectiveai-ensemble 2>&1 | tee /dev/tty | grep -q "already uploaded"; then
            if [ ${PIPESTATUS[0]} -ne 0 ]; then
                log_error "Failed to publish objectiveai-ensemble"
                exit 1
            fi
        else
            log_warn "objectiveai-ensemble v$VERSION already published, skipping"
        fi
        log_info "objectiveai-ensemble published successfully"
    fi

    # Wait for crates.io to index (configurable via CRATES_IO_WAIT_SECONDS)
    WAIT_TIME=${CRATES_IO_WAIT_SECONDS:-30}
    log_info "Waiting $WAIT_TIME seconds for crates.io to index..."
    if [ "$DRY_RUN" = false ]; then
        sleep "$WAIT_TIME"
    fi

    # Publish objectiveai-ensemble-js
    log_info "Publishing objectiveai-ensemble-js..."
    if [ "$DRY_RUN" = true ]; then
        log_warn "[DRY-RUN] Would run: cargo publish -p objectiveai-ensemble-js"
    else
        if ! cargo publish -p objectiveai-ensemble-js 2>&1 | tee /dev/tty | grep -q "already uploaded"; then
            if [ ${PIPESTATUS[0]} -ne 0 ]; then
                log_error "Failed to publish objectiveai-ensemble-js"
                exit 1
            fi
        else
            log_warn "objectiveai-ensemble-js v$VERSION already published, skipping"
        fi
        log_info "objectiveai-ensemble-js published successfully"
    fi

    log_info "All crates published to crates.io"
    echo ""
fi

# Exit if crates-only
if [ "$CRATES_ONLY" = true ]; then
    log_info "Crates-only mode: Skipping npm publish"
    echo ""
    log_info "Done!"
    exit 0
fi

# Step 5: Build WASM package
log_info "Step 5: Building WASM package with wasm-pack..."
if [ "$DRY_RUN" = true ]; then
    log_warn "[DRY-RUN] Would run: wasm-pack build --target web objectiveai-ensemble-js"
else
    wasm-pack build --target web objectiveai-ensemble-js
    log_info "WASM package built successfully"
fi
echo ""

# Step 6: Modify package.json
log_info "Step 6: Modifying package.json..."
PKG_DIR="objectiveai-ensemble-js/pkg"
PKG_JSON="$PKG_DIR/package.json"

if [ "$DRY_RUN" = true ]; then
    log_warn "[DRY-RUN] Would modify: $PKG_JSON"
    log_warn "[DRY-RUN] Would add: description, repository, homepage, keywords, author"
else
    # Check if package.json exists
    if [ ! -f "$PKG_JSON" ]; then
        log_error "package.json not found at $PKG_JSON. wasm-pack build may have failed."
        exit 1
    fi

    # Use Node.js to safely modify the package.json
    node << 'EOF'
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
EOF
    if [ $? -ne 0 ]; then
        log_error "Failed to update package.json"
        exit 1
    fi
    log_info "package.json updated successfully"
fi
echo ""

# Step 7: Copy LICENSE
log_info "Step 7: Copying LICENSE to pkg directory..."
if [ "$DRY_RUN" = true ]; then
    log_warn "[DRY-RUN] Would copy: LICENSE -> $PKG_DIR/LICENSE"
else
    if [ ! -f "LICENSE" ]; then
        log_error "LICENSE file not found in repository root"
        exit 1
    fi
    if [ ! -d "$PKG_DIR" ]; then
        log_error "Package directory $PKG_DIR not found"
        exit 1
    fi
    cp LICENSE "$PKG_DIR/LICENSE"
    log_info "LICENSE copied successfully"
fi
echo ""

# Step 8: Publish to npm
log_info "Step 8: Publishing to npm..."
if [ "$DRY_RUN" = true ]; then
    log_warn "[DRY-RUN] Would run: npm publish --access public (from $PKG_DIR)"
else
    cd "$PKG_DIR" || { log_error "Failed to change to directory $PKG_DIR"; exit 1; }
    npm publish --access public
    cd ../..
    log_info "@objectiveai/ensemble published to npm successfully"
fi
echo ""

# Step 9: Create git tag
if [ "$SKIP_GIT_TAG" = false ]; then
    log_info "Step 9: Creating git tag..."
    TAG="v$VERSION"

    if [ "$DRY_RUN" = true ]; then
        log_warn "[DRY-RUN] Would create tag: $TAG"
        log_warn "[DRY-RUN] Would run: git tag $TAG && git push origin $TAG"
    else
        # Check if tag already exists
        if git rev-parse "$TAG" >/dev/null 2>&1; then
            log_warn "Tag $TAG already exists, skipping tag creation"
        else
            if ! git tag "$TAG"; then
                log_error "Failed to create git tag $TAG"
                exit 1
            fi
            if ! git push origin "$TAG"; then
                log_error "Failed to push git tag $TAG"
                exit 1
            fi
            log_info "Git tag $TAG created and pushed"
        fi
    fi
    echo ""
else
    log_warn "Step 9: Skipping git tag creation (--skip-git-tag)"
    echo ""
fi

# Success!
echo "================================================"
if [ "$DRY_RUN" = true ]; then
    log_info "DRY-RUN COMPLETE"
    echo ""
    log_info "To actually publish, run:"
    echo "  ./publish.sh --no-dry-run"
else
    log_info "PUBLISH COMPLETE!"
    echo ""
    log_info "Published:"
    if [ "$NPM_ONLY" = false ]; then
        echo "  - objectiveai-ensemble v$VERSION to crates.io"
        echo "  - objectiveai-ensemble-js v$VERSION to crates.io"
    fi
    if [ "$CRATES_ONLY" = false ]; then
        echo "  - @objectiveai/ensemble v$VERSION to npm"
    fi
fi
echo "================================================"
