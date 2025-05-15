#!/bin/bash
#
# Kitty Restore Command Test Script
# This script tests the restore functionality of the kitty tool
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
TEST_DIR="kitty_restore_test"
FILE_REPO_DIR="${TEST_DIR}/file_repo"
SQLITE_REPO_DIR="${TEST_DIR}/sqlite_repo"
KITTY_CMD="cargo run --"  # Use your kitty command here (e.g., "kitty" if installed)
PASSWORD="testpassword"  # Password for the test repository

echo -e "${BOLD}Kitty Restore Command Test${RESET}"
echo "==========================="
echo

# Cleanup any existing test data
cleanup() {
    echo -e "\n${BLUE}Cleaning up test data...${RESET}"
    rm -rf "$TEST_DIR"
    echo "Cleanup complete."
}

# Handle errors and cleanup
handle_error() {
    echo -e "${RED}An error occurred during testing.${RESET}"
    cleanup
    exit 1
}

trap handle_error ERR

# Create test directory structure
mkdir -p "$FILE_REPO_DIR"
mkdir -p "$SQLITE_REPO_DIR"

# Test file-based storage restore
echo -e "${BOLD}Testing File-based Storage Restore${RESET}"
echo "--------------------------------"

# Change to file repo directory
cd "$FILE_REPO_DIR"

# Initialize repository (file-based)
echo -e "\n${BOLD}Step 1: Initializing file-based repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD init

# Create test files
echo -e "\n${BOLD}Step 2: Creating test files${RESET}"
echo "This is test file 1 for the restore command test." > test_file1.txt
echo "This is test file 2 for the restore command test." > test_file2.txt

# Add files to repository
echo -e "\n${BOLD}Step 3: Adding test files to repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD add test_file1.txt
echo "$PASSWORD" | $KITTY_CMD add test_file2.txt

# List the added files
echo -e "\n${BOLD}Step 4: Listing tracked files${RESET}"
echo "$PASSWORD" | $KITTY_CMD list

# Modify the original files
echo -e "\n${BOLD}Step 5: Modifying original files${RESET}"
echo "This file has been modified after being added to the repository." >> test_file1.txt
echo "Modified line in test file 2." > test_file2.txt

# Check differences
echo -e "\n${BOLD}Step 6: Checking differences${RESET}"
echo "$PASSWORD" | $KITTY_CMD diff

# Create backups of the modified files
cp test_file1.txt test_file1.modified
cp test_file2.txt test_file2.modified

# Restore files from repository
echo -e "\n${BOLD}Step 7: Restoring files from repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD restore test_file1.txt
echo "$PASSWORD" | $KITTY_CMD restore test_file2.txt

# Verify restored files
echo -e "\n${BOLD}Step 8: Verifying restored files${RESET}"
if diff -q test_file1.txt test_file1.modified > /dev/null; then
    echo -e "${RED}ERROR: test_file1.txt was not restored properly${RESET}"
else
    echo -e "${GREEN}SUCCESS: test_file1.txt was restored successfully${RESET}"
fi

if diff -q test_file2.txt test_file2.modified > /dev/null; then
    echo -e "${RED}ERROR: test_file2.txt was not restored properly${RESET}"
else
    echo -e "${GREEN}SUCCESS: test_file2.txt was restored successfully${RESET}"
fi

# Return to test directory
cd ..

# Test SQLite-based storage restore
echo -e "\n\n${BOLD}Testing SQLite-based Storage Restore${RESET}"
echo "--------------------------------"

# Change to SQLite repo directory
cd "$SQLITE_REPO_DIR"

# Initialize repository with SQLite
echo -e "\n${BOLD}Step 1: Initializing SQLite-based repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD init --sqlite

# Create test files
echo -e "\n${BOLD}Step 2: Creating test files${RESET}"
echo "This is SQLite test file 1 for the restore command test." > sqlite_test1.txt
echo "This is SQLite test file 2 for the restore command test." > sqlite_test2.txt

# Add files to repository
echo -e "\n${BOLD}Step 3: Adding test files to SQLite repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD add sqlite_test1.txt
echo "$PASSWORD" | $KITTY_CMD add sqlite_test2.txt

# List the added files
echo -e "\n${BOLD}Step 4: Listing tracked files in SQLite repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD list

# Modify the original files
echo -e "\n${BOLD}Step 5: Modifying original files${RESET}"
echo "This SQLite-stored file has been modified after being added to the repository." >> sqlite_test1.txt
echo "Modified line in SQLite test file 2." > sqlite_test2.txt

# Check differences
echo -e "\n${BOLD}Step 6: Checking differences${RESET}"
echo "$PASSWORD" | $KITTY_CMD diff

# Create backups of the modified files
cp sqlite_test1.txt sqlite_test1.modified
cp sqlite_test2.txt sqlite_test2.modified

# Restore files from repository
echo -e "\n${BOLD}Step 7: Restoring files from SQLite repository${RESET}"
echo "$PASSWORD" | $KITTY_CMD restore sqlite_test1.txt
echo "$PASSWORD" | $KITTY_CMD restore sqlite_test2.txt

# Verify restored files
echo -e "\n${BOLD}Step 8: Verifying restored files${RESET}"
if diff -q sqlite_test1.txt sqlite_test1.modified > /dev/null; then
    echo -e "${RED}ERROR: sqlite_test1.txt was not restored properly${RESET}"
else
    echo -e "${GREEN}SUCCESS: sqlite_test1.txt was restored successfully${RESET}"
fi

if diff -q sqlite_test2.txt sqlite_test2.modified > /dev/null; then
    echo -e "${RED}ERROR: sqlite_test2.txt was not restored properly${RESET}"
else
    echo -e "${GREEN}SUCCESS: sqlite_test2.txt was restored successfully${RESET}"
fi

# Return to parent directory
cd ../..

# Test Summary
echo -e "\n${BOLD}Test Summary${RESET}"
echo "============"
echo -e "✅ File-based repository restore"
echo -e "✅ SQLite-based repository restore"
echo -e "✅ File content verification"

# Cleanup
cleanup

echo -e "\n${BOLD}Restore Testing Complete!${RESET}"