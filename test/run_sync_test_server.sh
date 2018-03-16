#!/bin/bash

source .env


export DATABASE_URL=$DATABASE_FOREIGN_URL
export FILE_STORAGE_PATH=/tmp/flash_sync
export FILE_READ_PATH=test/media/foreign
export FLASH_PORT=3001
export LOG_FILE=/tmp/flash_sync/log
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

echo "Database: $DATABASE_URL" >> $LOG_FILE


# Set up database
if ! diesel database reset >> $LOG_FILE; then
    echo "Database setup failed"
    cat $LOG_FILE
    exit -1
fi

if [ "$1" == "" ]; then
    # Create a fifo file for writing the output of flash
    FIFO_NAME=/tmp/flash_sync/fifo
    mkfifo $FIFO_NAME
    # Run flash itself
    if [ -e target/debug/flash ] ; then
        target/debug/flash < /dev/null &> $FIFO_NAME &
        # Get the pid of the program so we can kill it later
        FLASH_PID=$!
    else
        echo "Flash executable does not exist, did you forget to build it"
        exit -1
    fi


    # Wait for the child process to get ready
    until grep -m 1 "ready" $FIFO_NAME > /dev/null; do sleep 0.1; done

    echo "{\"pid\": \"$FLASH_PID\"}"
elif [ "$1" == "gdb" ]; then
    gdb target/debug/flash
else
    echo "Unrecognised subcommand: $1, expected '' or gdb"
    exit -1
fi
