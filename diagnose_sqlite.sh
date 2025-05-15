#!/bin/bash
#
# Kitty SQLite Storage Diagnostic Script
# This script helps diagnose issues with kitty's SQLite storage

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

echo -e "${BOLD}Kitty SQLite Storage Diagnostic Tool${RESET}"
echo "============================================="
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

# Check the storage type
if [ -f "$STORAGE_TYPE_FILE" ]; then
    STORAGE=$(cat "$STORAGE_TYPE_FILE")
    if [ "$STORAGE" != "sqlite" ]; then
        echo -e "${YELLOW}Warning: Repository is not using SQLite storage.${RESET}"
        echo "Current storage type: $STORAGE"
        echo
        read -p "Would you like to convert to SQLite storage? (y/n): " CONVERT
        if [[ $CONVERT == "y" || $CONVERT == "Y" ]]; then
            echo "sqlite" > "$STORAGE_TYPE_FILE"
            echo -e "${GREEN}Storage type changed to SQLite.${RESET}"
        else
            echo "Continuing diagnostics without conversion."
        fi
    else
        echo -e "${GREEN}Repository is using SQLite storage.${RESET}"
    fi
else
    echo -e "${YELLOW}Warning: No storage type file found.${RESET}"
    echo "Creating storage type file with SQLite as the default."
    echo "sqlite" > "$STORAGE_TYPE_FILE"
fi

# Check if the SQLite database exists
if [ ! -f "$SQLITE_DB" ]; then
    echo -e "${RED}Error: SQLite database not found at $SQLITE_DB${RESET}"
    exit 1
fi

echo -e "\n${BOLD}SQLite Database Structure:${RESET}"
echo "-------------------------"
sqlite3 "$SQLITE_DB" ".schema"

# Check if the files table has a content column
if sqlite3 "$SQLITE_DB" ".schema files" | grep -q "content BLOB"; then
    echo -e "\nFile content storage: ${GREEN}Enabled in database${RESET}"
else
    echo -e "\nFile content storage: ${RED}Not enabled in database${RESET}"
    echo "The database schema is missing the 'content' column in the 'files' table."
    echo "This may cause diff and restore operations to fail."
fi

echo -e "\n${BOLD}Checking Repository Data:${RESET}"
echo "-------------------------"
REPO_COUNT=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM repository")
echo -e "Repository records: ${BLUE}$REPO_COUNT${RESET}"
if [ "$REPO_COUNT" -eq 0 ]; then
    echo -e "${RED}Error: No repository records found.${RESET}"
else
    sqlite3 "$SQLITE_DB" "SELECT id, created_at FROM repository" 
fi

echo -e "\n${BOLD}Checking Tracked Files:${RESET}"
echo "-------------------------"
FILE_COUNT=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM files")
echo -e "Tracked files: ${BLUE}$FILE_COUNT${RESET}"

if [ "$FILE_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}Warning: No files are being tracked.${RESET}"
else
    echo -e "\n${BOLD}Tracked File Paths:${RESET}"
    sqlite3 "$SQLITE_DB" "SELECT original_path, repo_path FROM files" | while read -r line; do
        ORIGINAL_PATH=$(echo "$line" | cut -d'|' -f1)
        REPO_PATH=$(echo "$line" | cut -d'|' -f2)
        
        echo -e "\nOriginal path: ${BLUE}$ORIGINAL_PATH${RESET}"
        echo -e "Repository path: ${BLUE}$REPO_PATH${RESET}"
        
        # Check if original file exists
        if [ -f "$ORIGINAL_PATH" ]; then
            echo -e "Original file exists: ${GREEN}Yes${RESET}"
        else
            echo -e "Original file exists: ${RED}No${RESET}"
        fi
        
        # Check if file content exists in database
        CONTENT_SIZE=$(sqlite3 "$SQLITE_DB" "SELECT length(content) FROM files WHERE repo_path='$REPO_PATH'")
        if [ -z "$CONTENT_SIZE" ]; then
            CONTENT_SIZE=0
        fi
        
        if [ "$CONTENT_SIZE" -gt 0 ]; then
            echo -e "File content in database: ${GREEN}Yes ($CONTENT_SIZE bytes)${RESET}"
        else
            echo -e "File content in database: ${RED}No${RESET}"
            echo -e "${YELLOW}Warning: This file has no content stored in the database.${RESET}"
        fi
        
        # Check if repository file exists on filesystem (legacy storage)
        FULL_REPO_PATH="${KITTY_REPO}/$REPO_PATH"
        if [ -f "$FULL_REPO_PATH" ]; then
            echo -e "Repository file in filesystem: ${GREEN}Yes${RESET}"
            echo -e "${YELLOW}Note: File exists both in database and filesystem.${RESET}"
        else
            echo -e "Repository file in filesystem: ${RED}No${RESET}"
            
            # Only show this warning if there's also no content in the database
            if [ "$CONTENT_SIZE" -eq 0 ]; then
                # Try to find the file in another location
                FOUND_PATH=$(find "$KITTY_REPO" -type f -name "$(basename "$REPO_PATH")" | head -1)
                if [ -n "$FOUND_PATH" ]; then
                    echo -e "Found similar file at: ${YELLOW}$FOUND_PATH${RESET}"
                    echo "You might need to update the path in the database."
                fi
            fi
        fi
    done
fi

echo -e "\n${BOLD}Diagnostics Complete${RESET}"
echo "======================="
echo -e "If you're having issues with the diff command, check that:"
echo "1. The original files exist at the paths stored in the database"
echo "2. Each file has content stored in the database (content column)"
echo "3. File paths are consistent between database entries"
echo
echo -e "${BOLD}SQLite Storage Troubleshooting:${RESET}"
echo "1. Make sure your database schema includes the 'content' column"
echo "2. If files are missing content, you may need to re-add them"
echo "3. You can verify content with: ${BLUE}sqlite3 $SQLITE_DB \"SELECT length(content) FROM files WHERE repo_path='your/path';\"${RESET}"
echo
echo -e "To fix file issues, you can use:"
echo -e "${YELLOW}sqlite3 $SQLITE_DB${RESET}"
echo -e "For path issues: ${BLUE}UPDATE files SET repo_path='correct/path' WHERE original_path='problem/path';${RESET}"
echo -e "For missing content: Re-add the file with the 'kitty add' command"
echo
echo "Note: When using SQLite storage, kitty stores file content directly in the database."
echo "You can safely delete the files in the .kitty/files directory as they are not used."
echo
echo "For other issues, please check the Kitty documentation or report a bug."