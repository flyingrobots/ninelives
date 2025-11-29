#!/usr/bin/env bash
# Scaffold a template test under tests/<name>.rs.
# Usage: scripts/scaffold-test.sh my_feature_test
# Notes: refuses unsafe names (slashes/dots), does not overwrite existing files,
# and generates an ignored placeholder test you must enable/implement.
set -euo pipefail

TEST_NAME="${1:-}"

if [ -z "$TEST_NAME" ]; then
    echo "Usage: scripts/scaffold-test.sh <test_name_snake_case>"
    echo "Example: scripts/scaffold-test.sh feature_x_behavior"
    exit 1
fi

if [[ "$TEST_NAME" == /* || "$TEST_NAME" == .* || "$TEST_NAME" == *"/"* || "$TEST_NAME" == *".."* ]]; then
    echo "Error: test name must be a simple snake_case identifier (no slashes, dots, or traversal)." >&2
    exit 1
fi

mkdir -p tests
FILE_PATH="tests/${TEST_NAME}.rs"
RUST_FN_NAME="${TEST_NAME}_placeholder"

if ! [[ "$RUST_FN_NAME" =~ ^[a-z_][a-z0-9_]*$ ]]; then
    echo "Error: generated function name '$RUST_FN_NAME' is not a valid Rust identifier." >&2
    exit 1
fi

if [ -f "$FILE_PATH" ]; then
    echo "Test file $FILE_PATH already exists."
    echo "To add a new test case, open it and add a #[tokio::test] async fn."
else
    echo "Creating $FILE_PATH..."
    cat <<-EOF > "$FILE_PATH"
	use ninelives::prelude::*;

	#[ignore]
	#[tokio::test]
	async fn ${RUST_FN_NAME}() {
	    // TODO: implement and remove #[ignore]
	    panic!("Test not yet implemented");
	}
	EOF
    echo "Created ignored placeholder test in $FILE_PATH"
fi

echo "Run this test with: cargo test --test ${TEST_NAME}"
