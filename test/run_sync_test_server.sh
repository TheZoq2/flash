#!/bin/bash

source .env

(>&2 echo "Building project")

if ! cargo build 2> build_err.log > build.log ; then
    echo "Cargo build failed"
    cat build_err.log
    cat build.log
    exit -1
fi
(>&2 echo "Done")



# Set to 1 if the script should use the enviroment variables for flash parameters
# rather than the default ones for testing
# Set to 1 if flash should be run in gdb rather than a orphaned child process. Set
# this option to debug the remote server
USE_GDB=0

export FLASH_PORT=3001

# Read commandline parameters
while getopts ":gp:" opt; do
    case $opt in
        g)
            USE_GDB=1
            ;;
        p)
            FLASH_PORT=${OPTARG}
            ;;
        \?)
            echo "Invalid option: -$opt" >&2
            echo ""
            echo " -g run with gdb rather than an orphaned child process"
            echo " -e use enviroment variables rather than default for testing"
            exit -1
            ;;
    esac
done

export DATABASE_URL="${DATABASE_FOREIGN_URL}_${FLASH_PORT}"
export FILE_STORAGE_PATH="/tmp/flash_sync_${FLASH_PORT}"
export FILE_READ_PATH=test/media/foreign

# Log data to this file
export LOG_FILE="${FILE_STORAGE_PATH}/log_${FLASH_PORT}"
# Use diesel as executable if an alternative is not specified
export DIESEL_EXE=${DIESEL_EXE:=diesel}


# Check if any processes are using the port. Fuser output is weird which is why 2>devnull and sed are needed
port_pid=$(fuser "${FLASH_PORT}/tcp" 2>/dev/null | sed -e 's/ //g')
if [ "$port_pid" != "" ] ; then
    # Check if the pid using the port is a flash instance
    while read -r pid; do
        if [ "$port_pid" == "$pid" ]; then
            echo "null"
            exit
        fi
    done <<< "$(pidof flash | sed -e 's/ /\n/g')"

    # None of the flash pids are the same as the user of 3001. Some other program is using it
    echo "Port 3001 is already in used and it is not used by a flash instance"
    exit -1
fi

# Create storage dir
if ! mkdir -p "${FILE_STORAGE_PATH}" > /dev/null; then
    echo "Folder creation failed"
    exit -1
fi

# Clear the folder target folder if it is non-empty
if [ -z "$(ls "${FILE_STORAGE_PATH}")" ]; then : ; else
    if ! rm -r ${FILE_STORAGE_PATH:?}/* > /dev/null; then
        echo "Failed to remove content of storage folder"
        exit -1
    fi
fi

echo "Database: $DATABASE_URL" >> "$LOG_FILE"


# Set up database
if ! diesel database reset >> "$LOG_FILE"; then
    echo "Database setup failed"
    cat "$LOG_FILE"
    exit -1
fi

if [[ $USE_GDB -ne 1 ]]; then
    # Create a fifo file for writing the output of flash
    FLASH_LOG=${FILE_STORAGE_PATH}/flash_log

    # Run flash itself
    if [ -e target/debug/flash ] ; then
        target/debug/flash < /dev/null &> "$FLASH_LOG" &
        # Get the pid of the program so we can kill it later
        FLASH_PID=$!
    else
        echo "Flash executable does not exist, did you forget to build it"
        exit -1
    fi


    # Wait for the child process to get ready
    until curl "localhost:${FLASH_PORT}/ping" > /dev/null 2> /dev/null ; do sleep 0.1; done

    echo "{\"pid\": \"$FLASH_PID\"}"
else
    gdb target/debug/flash
fi
