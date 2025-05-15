#!/bin/bash
#
# Kitty SQLite Migration Script
# This script helps migrate existing SQLite-based repositories to store file content in the database
#

set -e

KITTY_REPO=".kitty"
SQLITE_DB="${KITTY_REPO}/kitty.db"
STORAGE_TYPE_FILE="${KITTY_REPO}/storage.type"

# Text formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

echo -e "${BOLD}Kitty SQLite Migration Tool${RESET}"
echo "============================="
echo

# Check if sqlite3 command is available
if ! command -v sqlite3 &> /dev/null; then
    echo -e "${RED}Error: sqlite3 command not found.${RESET}"
    echo "Please install SQLite to use this migration tool."
    exit 1
fi

# Check if the kitty repository exists
if [ ! -d "$KITTY_REPO" ]; then
    echo -e "${RED}Error: Kitty repository not found.${RESET}"
    echo "Make sure you're running this script from the directory containing the .kitty repository."
    exit 1
fi

# Check the storage type
if [ ! -f "$STORAGE_TYPE_FILE" ]; then
    echo -e "${RED}Error: Storage type file not found.${RESET}"
    echo "Your repository may be corrupt or using an old version format."
    exit 1
fi

STORAGE=$(cat "$STORAGE_TYPE_FILE")
if [ "$STORAGE" != "sqlite" ]; then
    echo -e "${RED}Error: Repository is not using SQLite storage.${RESET}"
    echo "Current storage type: $STORAGE"
    echo "This migration is only for SQLite-based repositories."
    exit 1
fi

# Check if the SQLite database exists
if [ ! -f "$SQLITE_DB" ]; then
    echo -e "${RED}Error: SQLite database not found at $SQLITE_DB${RESET}"
    exit 1
fi

echo -e "Checking database schema..."
# Check if the files table has a content column
if sqlite3 "$SQLITE_DB" ".schema files" | grep -q "content BLOB"; then
    echo -e "${GREEN}Content column already exists in files table.${RESET}"
    echo "No schema migration needed."
else
    echo -e "${YELLOW}Content column missing from files table.${RESET}"
    echo "Altering database schema to add content column..."
    
    # Add the content column to the files table
    sqlite3 "$SQLITE_DB" "ALTER TABLE files ADD COLUMN content BLOB;"
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Schema migration successful.${RESET}"
    else
        echo -e "${RED}Failed to alter database schema.${RESET}"
        exit 1
    fi
fi

echo -e "\n${BOLD}Migrating file content to database...${RESET}"
echo "This will read file content from the filesystem and store it in the database."

# Get all file records
FILES=$(sqlite3 "$SQLITE_DB" "SELECT id, original_path, repo_path FROM files;" -separator "|")

TOTAL_FILES=$(echo "$FILES" | wc -l)
MIGRATED=0
FAILED=0

echo "Found $TOTAL_FILES files to migrate."

while IFS="|" read -r ID ORIGINAL_PATH REPO_PATH; do
    if [ -z "$ID" ]; then
        continue
    fi
    
    echo -e "\nProcessing: ${BLUE}$REPO_PATH${RESET}"
    
    # Check if content already exists in the database
    CONTENT_SIZE=$(sqlite3 "$SQLITE_DB" "SELECT length(content) FROM files WHERE id=$ID;")
    if [ -z "$CONTENT_SIZE" ]; then
        CONTENT_SIZE=0
    fi
    
    if [ "$CONTENT_SIZE" -gt 0 ]; then
        echo -e "  ${GREEN}File already has content in database ($CONTENT_SIZE bytes)${RESET}"
        ((MIGRATED++))
        continue
    fi
    
    # Try to read from the filesystem
    FILE_PATH="${KITTY_REPO}/$REPO_PATH"
    if [ -f "$FILE_PATH" ]; then
        echo -e "  ${BLUE}Reading file from: $FILE_PATH${RESET}"
        
        # Store the file content in the database
        # We need to use the SQLite .import command with a temporary file to handle binary data properly
        TEMP_FILE=$(mktemp)
        cp "$FILE_PATH" "$TEMP_FILE"
        
        # Import the file content into the database
        echo ".open $SQLITE_DB" > /tmp/sqlite_commands.sql
        echo "UPDATE files SET content = readfile('$TEMP_FILE') WHERE id = $ID;" >> /tmp/sqlite_commands.sql
        
        sqlite3 < /tmp/sqlite_commands.sql
        
        if [ $? -eq 0 ]; then
            # Verify the content was stored
            NEW_CONTENT_SIZE=$(sqlite3 "$SQLITE_DB" "SELECT length(content) FROM files WHERE id=$ID;")
            echo -e "  ${GREEN}Successfully migrated file to database ($NEW_CONTENT_SIZE bytes)${RESET}"
            ((MIGRATED++))
        else
            echo -e "  ${RED}Failed to store file content in database${RESET}"
            ((FAILED++))
        fi
        
        # Clean up
        rm -f "$TEMP_FILE"
        rm -f /tmp/sqlite_commands.sql
    else
        echo -e "  ${RED}File not found in filesystem: $FILE_PATH${RESET}"
        ((FAILED++))
    fi
done <<< "$FILES"

echo -e "\n${BOLD}Migration Summary${RESET}"
echo "==================="
echo -e "Total files: ${BLUE}$TOTAL_FILES${RESET}"
echo -e "Successfully migrated: ${GREEN}$MIGRATED${RESET}"
echo -e "Failed to migrate: ${RED}$FAILED${RESET}"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All files were successfully migrated to the database.${RESET}"
    echo "You can now safely delete the files in the .kitty/files directory."
    echo -e "To clean up, you can run: ${BLUE}rm -rf .kitty/files${RESET}"
else
    echo -e "\n${YELLOW}Some files could not be migrated.${RESET}"
    echo "You may need to re-add these files using 'kitty add <path>'."
    echo "Do not delete the .kitty/files directory until all files are migrated."
fi

echo -e "\n${BOLD}Migration Complete${RESET}"