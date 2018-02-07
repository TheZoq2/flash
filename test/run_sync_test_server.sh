#!/bin/bash

source .env

export DATABASE_URL=$DATABASE_FOREIGN_URL
export FILE_STORAGE_PATH=/tmp/flash_sync
export FILE_READ_PATH=test/media/foreign
export FLASH_PORT=3001
export LOG_FILE=/tmp/flash_sync/log
export DIESEL_EXE=${DIESEL_EXE:=diesel}


# Create storage dir
if ! mkdir -p ${FILE_STORAGE_PATH} > /dev/null; then
    echo "Folder creation failed"
    exit -1
fi

# Clear the folder target folder if it is non-empty
if [ -z "$(ls ${FILE_STORAGE_PATH})" ]; then : ; else
    if ! rm -r ${FILE_STORAGE_PATH:?}/* > /dev/null; then
        echo "Failed to remove content of storage folder"
        exit -1
    fi
fi


#e Set up database
if ! diesel database reset > $LOG_FILE; then
    echo "Database setup failed"
    cat $LOG_FILE
    exit -1
fi


# Create a fifo file for writing the output of flash
FIFO_NAME=/tmp/flash_sync/fifo
mkfifo $FIFO_NAME
# Run flash itself
target/debug/flash < /dev/null &> $FIFO_NAME &
# Get the pid of the program so we can kill it later
FLASH_PID=$!

# Wait for the child process to get ready
until grep -m 1 "ready" $FIFO_NAME > /dev/null; do sleep 0.1; done

echo "$FLASH_PID"

