#!/usr/bin/env bash
set -euo pipefail

TEST_NAME="${1:-}"

if [ -z "$TEST_NAME" ]; then
    echo "Usage: scripts/scaffold-test.sh <test_name_snake_case>"
    echo "Example: scripts/scaffold-test.sh feature_x_behavior"
    exit 1
fi

FILE_PATH="tests/${TEST_NAME}.rs"

if [ -f "$FILE_PATH" ]; then
    echo "Test file $FILE_PATH already exists."
    echo "To add a new test case, open it and add a #[tokio::test] async fn."
else
    echo "Creating $FILE_PATH..."
    cat <<EOF > "$FILE_PATH"
use ninelives::prelude::*;

#[tokio::test]
async fn ${TEST_NAME}_should_fail_initially() {
    // TODO: Implement the setup for your test
    // This is a scaffold. Replace this with your actual test logic.
    let result = "not implemented";
    
    // Assert failure to demonstrate the test is running and failing as expected
    assert_eq!(result, "expected value", "Test should fail until implemented");
}
EOF
    echo "Created failing test in $FILE_PATH"
fi

echo "Run this test with: cargo test --test ${TEST_NAME}"
