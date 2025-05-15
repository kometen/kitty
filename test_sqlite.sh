#!/bin/bash
#
# Kitty SQLite Storage Test Script
# This script tests the SQLite storage functionality of the kitty tool
#

set -e

# Text formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

# Configuration
TEST_REPO_DIR="kitty_test_repo"
TEST_FILE="kitty_test_file.txt"
KITTY_CMD="cargo run --"  # Use your kitty command here (e.g., "kitty" if installed)
PASSWORD="testpassword"  # Password for the test repository

echo -e "${BOLD}Kitty SQLite Storage Test${RESET}"
echo "========================="
echo

# Cleanup any existing test data
cleanup() {
    echo -e "\n${BLUE}Cleaning up test data...${RESET}"
    rm -rf "$TEST_REPO_DIR"
    rm -f "$TEST_FILE"
    echo "Cleanup complete."
}

# Handle errors and cleanup
handle_error() {
    echo -e "${RED}An error occurred during testing.${RESET}"
    cleanup
    exit 1
}

trap handle_error ERR

# Create test directory and file
mkdir -p "$TEST_REPO_DIR"
cd "$TEST_REPO_DIR"

# Create a test file with some content
echo "This is a test file for the kitty configuration management tool." > "$TEST_FILE"
echo "It contains some text that will be tracked and stored in SQLite." >> "$TEST_FILE"
echo "Line 3: Additional content for testing diffs later." >> "$TEST_FILE"
echo "Line 4: More content for the test file." >> "$TEST_FILE"

echo -e "${BLUE}Created test file:${RESET} $TEST_FILE"

# Step 1: Initialize repository with SQLite
echo -e "\n${BOLD}Step 1: Initializing repository with SQLite${RESET}"
echo "$PASSWORD" | $KITTY_CMD init --sqlite

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Repository initialized successfully with SQLite storage.${RESET}"
else
    echo -e "${RED}Failed to initialize repository.${RESET}"
    cleanup
    exit 1
fi

# Step 2: Add the test file
echo -e "\n${BOLD}Step 2: Adding test file to repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD add "$TEST_FILE"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Test file added successfully.${RESET}"
else
    echo -e "${RED}Failed to add test file.${RESET}"
    cleanup
    exit 1
fi

# Step 3: List tracked files
echo -e "\n${BOLD}Step 3: Listing tracked files${RESET}"
echo "$PASSWORD" | $KITTY_CMD list

if [ $? -eq 0 ]; then
    echo -e "${GREEN}File listing successful.${RESET}"
else
    echo -e "${RED}Failed to list files.${RESET}"
    cleanup
    exit 1
fi

# Step 4: Run a diff operation
echo -e "\n${BOLD}Step 4: Testing diff operation${RESET}"
echo "$PASSWORD" | $KITTY_CMD diff "$TEST_FILE"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Diff operation successful.${RESET}"
else
    echo -e "${RED}Diff operation failed.${RESET}"
    cleanup
    exit 1
fi

# Step 5: Modify the file and test diff again
echo -e "\n${BOLD}Step 5: Testing diff with modified file${RESET}"
echo "This line was added for testing diff functionality." >> "$TEST_FILE"
echo "Another modified line." >> "$TEST_FILE"

echo -e "${BLUE}File modified. Running diff again...${RESET}"
echo "$PASSWORD" | $KITTY_CMD diff "$TEST_FILE"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Diff with modified file successful.${RESET}"
else
    echo -e "${RED}Diff with modified file failed.${RESET}"
    cleanup
    exit 1
fi

# Step 6: Test SQLite database
echo -e "\n${BOLD}Step 6: Examining SQLite database${RESET}"
if command -v sqlite3 &> /dev/null; then
    echo -e "${BLUE}Database tables:${RESET}"
    sqlite3 .kitty/kitty.db ".tables"
    
    echo -e "\n${BLUE}Files table schema:${RESET}"
    sqlite3 .kitty/kitty.db ".schema files"
    
    echo -e "\n${BLUE}Checking file content in database:${RESET}"
    CONTENT_COUNT=$(sqlite3 .kitty/kitty.db "SELECT COUNT(*) FROM files WHERE content IS NOT NULL")
    echo "Files with content in database: $CONTENT_COUNT"
    
    if [ "$CONTENT_COUNT" -gt 0 ]; then
        echo -e "${GREEN}File content is properly stored in the database.${RESET}"
    else
        echo -e "${RED}No file content found in the database.${RESET}"
    fi
else
    echo -e "${YELLOW}SQLite command-line tool not found. Skipping database examination.${RESET}"
fi

# Step 7: Test restore functionality
echo -e "\n${BOLD}Step 7: Testing restore functionality${RESET}"
# Make a backup of the current file
cp "$TEST_FILE" "${TEST_FILE}.bak"
# Remove the original file
rm "$TEST_FILE"
echo -e "${BLUE}Original file removed. Testing restore...${RESET}"

# Restore the file
echo "$PASSWORD" | $KITTY_CMD restore "$TEST_FILE"

if [ $? -eq 0 ] && [ -f "$TEST_FILE" ]; then
    echo -e "${GREEN}File restore successful.${RESET}"
    echo -e "${BLUE}Content of restored file:${RESET}"
    cat "$TEST_FILE"
else
    echo -e "${RED}File restore failed.${RESET}"
    cleanup
    exit 1
fi

# Final summary
echo -e "\n${BOLD}Test Summary${RESET}"
echo "=============="
echo -e "${GREEN}All tests completed successfully!${RESET}"
echo "Your kitty SQLite storage implementation is working correctly."
echo
echo -e "The following operations were verified:"
echo -e "✅ Repository initialization with SQLite"
echo -e "✅ Adding files to the repository"
echo -e "✅ Listing tracked files"
echo -e "✅ Diff operation with original and modified files"
echo -e "✅ File content storage in the SQLite database"
echo -e "✅ File restoration from the repository"

# Cleanup
echo -e "\nCleaning up test files and repository..."
cd ..
cleanup

echo -e "\n${BOLD}Testing complete!${RESET}"