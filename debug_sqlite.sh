#!/bin/bash
#
# Kitty SQLite Storage Debug Tool
# This script provides detailed information about the SQLite storage in kitty

set -e

KITTY_REPO=".kitty"
SQLITE_DB="${KITTY_REPO}/kitty.db"

# Text formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

echo -e "${BOLD}Kitty SQLite Storage Debug Tool${RESET}"
echo "==============================="
echo

# Check if sqlite3 command is available
if ! command -v sqlite3 &> /dev/null; then
    echo -e "${RED}Error: sqlite3 command not found.${RESET}"
    echo "Please install SQLite to use this diagnostic tool."
    exit 1
fi

# Check if the kitty repository exists
if [ ! -d "$KITTY_REPO" ]; then
    echo -e "${RED}Error: Kitty repository not found.${RESET}"
    echo "Make sure you're running this script from the directory containing the .kitty repository."
    exit 1
fi

# Check if the SQLite database exists
if [ ! -f "$SQLITE_DB" ]; then
    echo -e "${RED}Error: SQLite database not found at $SQLITE_DB${RESET}"
    exit 1
fi

echo -e "${BOLD}Database Information:${RESET}"
echo "---------------------"
DB_SIZE=$(du -h "$SQLITE_DB" | cut -f1)
echo -e "Database size: ${BLUE}$DB_SIZE${RESET}"

# Database structure details
echo -e "\n${BOLD}Database Schema:${RESET}"
echo "---------------"
sqlite3 "$SQLITE_DB" ".schema"

# Check if content column exists
if sqlite3 "$SQLITE_DB" ".schema files" | grep -q "content BLOB"; then
    echo -e "\n${GREEN}✓ Content column exists in files table${RESET}"
else
    echo -e "\n${RED}✗ Content column missing from files table${RESET}"
    echo "Run the migrate_sqlite.sh script to add the content column."
fi

# Repository information
echo -e "\n${BOLD}Repository Details:${RESET}"
echo "------------------"
REPO_COUNT=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM repository")
if [ "$REPO_COUNT" -gt 0 ]; then
    echo -e "${GREEN}✓ Repository record found${RESET}"
    sqlite3 "$SQLITE_DB" "SELECT id, created_at FROM repository"
else
    echo -e "${RED}✗ No repository record found${RESET}"
fi

# File statistics
echo -e "\n${BOLD}File Statistics:${RESET}"
echo "---------------"
FILE_COUNT=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM files")
echo -e "Total files tracked: ${BLUE}$FILE_COUNT${RESET}"

# Check content storage
FILES_WITH_CONTENT=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM files WHERE content IS NOT NULL")
echo -e "Files with content stored: ${BLUE}$FILES_WITH_CONTENT${RESET}"

if [ "$FILES_WITH_CONTENT" -lt "$FILE_COUNT" ]; then
    echo -e "${YELLOW}Warning: Some files ($((FILE_COUNT - FILES_WITH_CONTENT))) don't have content stored in the database${RESET}"
fi

# Content size statistics
echo -e "\n${BOLD}Content Size Statistics:${RESET}"
echo "-----------------------"
AVG_SIZE=$(sqlite3 "$SQLITE_DB" "SELECT CAST(AVG(length(content)) AS INTEGER) FROM files WHERE content IS NOT NULL")
MAX_SIZE=$(sqlite3 "$SQLITE_DB" "SELECT MAX(length(content)) FROM files WHERE content IS NOT NULL")
MIN_SIZE=$(sqlite3 "$SQLITE_DB" "SELECT MIN(length(content)) FROM files WHERE content IS NOT NULL")

echo -e "Average content size: ${BLUE}$AVG_SIZE bytes${RESET}"
echo -e "Maximum content size: ${BLUE}$MAX_SIZE bytes${RESET}"
echo -e "Minimum content size: ${BLUE}$MIN_SIZE bytes${RESET}"

# Detailed file information
echo -e "\n${BOLD}Detailed File Information (up to 10 files):${RESET}"
echo "-------------------------------------"
sqlite3 "$SQLITE_DB" "
.mode column
.headers on
SELECT 
  id, 
  substr(original_path, 1, 40) as original_path, 
  substr(repo_path, 1, 20) as repo_path, 
  length(content) as content_bytes
FROM files 
LIMIT 10;"

echo -e "\n${BOLD}Files without Content:${RESET}"
echo "--------------------"
FILES_WITHOUT_CONTENT=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM files WHERE content IS NULL")

if [ "$FILES_WITHOUT_CONTENT" -gt 0 ]; then
    echo -e "${YELLOW}Found $FILES_WITHOUT_CONTENT files without content:${RESET}"
    sqlite3 "$SQLITE_DB" "
    .mode column
    .headers on
    SELECT 
      id, 
      substr(original_path, 1, 60) as original_path, 
      substr(repo_path, 1, 30) as repo_path 
    FROM files 
    WHERE content IS NULL 
    LIMIT 10;"
    
    if [ "$FILES_WITHOUT_CONTENT" -gt 10 ]; then
        echo "(Showing first 10 of $FILES_WITHOUT_CONTENT)"
    fi
else
    echo -e "${GREEN}✓ All files have content stored in the database${RESET}"
fi

# Check repo_path format
echo -e "\n${BOLD}Repository Path Format:${RESET}"
echo "----------------------"
FILES_WITH_FILES_PREFIX=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM files WHERE repo_path LIKE 'files/%'")
echo -e "Files with 'files/' prefix: ${BLUE}$FILES_WITH_FILES_PREFIX${RESET}"

if [ "$FILES_WITH_FILES_PREFIX" -gt 0 ] && [ "$FILE_COUNT" -eq "$FILES_WITH_FILES_PREFIX" ]; then
    echo -e "${YELLOW}Note: All files have 'files/' prefix in repo_path${RESET}"
    echo "This prefix is currently required for compatibility, don't change it."
elif [ "$FILES_WITH_FILES_PREFIX" -gt 0 ]; then
    echo -e "${YELLOW}Warning: Some files have 'files/' prefix and others don't${RESET}"
    echo "This mixed format may cause issues with the diff and restore commands."
fi

echo -e "\n${BOLD}Debug Complete${RESET}"
echo -e "Run ${BLUE}sqlite3 $SQLITE_DB${RESET} to manually inspect the database."
echo -e "Use ${BLUE}.tables${RESET} to list tables and ${BLUE}.schema [table]${RESET} to see table structure."
echo -e "Example: ${BLUE}SELECT id, original_path, length(content) FROM files;${RESET}"