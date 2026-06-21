#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <version-tag> (e.g., v0.1.0-beta1)" >&2
    exit 1
fi

TAG_VERSION="$1"
# Ensure it starts with v
if [[ ! "$TAG_VERSION" =~ ^v ]]; then
    echo "Error: version-tag must start with 'v' (e.g., v0.1.0-beta1)" >&2
    exit 1
fi

# Strip the leading 'v'
STRIPPED_VERSION="${TAG_VERSION#v}"

# Extract version from Cargo.toml
CARGO_VERSION=$(grep -m 1 "^version = " Cargo.toml | awk -F '"' '{print $2}')

if [ "$STRIPPED_VERSION" != "$CARGO_VERSION" ]; then
    echo "Error: Version mismatch! Git tag version ($STRIPPED_VERSION) does not match Cargo.toml version ($CARGO_VERSION)" >&2
    exit 1
fi

echo "=== 1. Checking code formatting ==="
cargo fmt --check

echo "=== 2. Running clippy ==="
cargo clippy --all-targets -- -D warnings

echo "=== 3. Running test suite ==="
cargo test

echo "=== 4. Building release binary ==="
cargo build --release

echo "=== 5. Packaging release ==="
rm -rf dist
mkdir -p dist

# Copy binary
cp target/release/nx9-auth dist/
strip dist/nx9-auth || true

# Create VERSION file
echo "$TAG_VERSION" > dist/VERSION

# Generate SHA256SUMS
cd dist
sha256sum nx9-auth VERSION > SHA256SUMS
cd ..

echo "=== Release $TAG_VERSION packaged successfully in dist/ ==="
ls -l dist/
