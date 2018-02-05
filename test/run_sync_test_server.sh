#!/bin/bash

export DATABASE_URL=postgres://flash:123456@localhost/flash_sync
export FILE_STORAGE_PATH=/tmp/flash_sync
export FILE_READ_PATH=./test/media/
export FLASH_PORT=3001

# Create storage dir
if ! mkdir -p ${FILE_STORAGE_PATH} > /dev/null; then
    echo "Folder creation failed"
    exit -1
fi


#e Set up database
if ! diesel database reset > /dev/null; then
    echo "Database setup failed"
    exit -1
fi

# Run flash itself
target/debug/flash > /dev/null &

pid=$!
echo "$pid"

