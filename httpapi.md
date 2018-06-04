
# /

Serves static files for the fontend. File paths are relative to `frontend/output`

# /list

Handles requests that deal with file lists. The `action` parameter specifies
what action to perform

## action="lists"

Returns a list of all current file lists

*Parameters*
 - None

*Returns*
Jsonified `file_list::FileListList`

## action="list_info"

Returns a `file_list_response::ListResponse` with information about the specified list

*Parameters*
 - `list_id`: ID of the target list

*Returns*
Jsonified `file_list_response::ListResponse`

## action="list_last_saved_index"

Returns the file ID of the last file that was saved to the database in the list.
Returns 0 if no file has been saved before

*Parameters*
 - `list_id`: ID of the target list

*Returns*
Integer


## action="get_data"

Returns a `file_request_handlers::FileData` struct containing the file path,
thumbnail_path and tags of the specified file

*Parameters*
 - `list_id`: ID of the target list
 - `index`: Index of the file in the database

*Returns*
Raw bytes

## action="get_file"

Returns the raw file data of the specified file

*Parameters*
 - `list_id`: ID of the target list
 - `index`: Index of the file in the database

*Returns*
String containing the path to the file

## action="get_filename"

Returns the filename of the specified file. If it is stored in the database,
only the filename is returned while the full path of an unsaved path is returned

*Parameters*
 - `list_id`: ID of the target list
 - `index`: Index of the file in the database

*Returns*
List string containing the file path or filename

*Notes*
This should probably be rewritten to return a path relative to `$FILE_READ_PATH`
for unsaved files to avoid exposing internal folder structures

## action="get_thumbnail"

Returns the raw data for the thumbnail of the specified file

*Parameters*
 - `list_id`: ID of the target list
 - `index`: Index of the file in the database

*Returns*
Raw bytes


## action="save"

Saves the specified file to the database. If the file was previously unsaved,
the file is added to the database, otherwise the file entry is updated.

The tags are set to whatever is specified in the query, and the `creation_time`
of the file is set to the current time

Responds with `"ok"`

*Params*
 - `list_id`: ID of the target list
 - `index`: Index of the file in the database
 - `tags`: JSON formated list of strings which should be the tags of the file

*Returns*
`"Ok"`



# /search

Performs a search for saved files. Replies with a `file_list_response::ListResponse`
with information about the list that is created from the search.

*Parameters*

- `query`: String containing the search query

*Returns*
Jsonified `file_list_response::ListResponse`

*Notes*
The format of the search query should probably be specified if it is not already


# /sync

Handles various requests relating to sync. Just `/sync` is unused

## /sync/sync

Starts a sync procedure with the specified foreign flash instance. Replies
with `"Sync done"` once the files have been synced successfully

*Params*
 - `foreign_url`: Url of the foreign flash instance to sync with

*Returns*
`"Sync done"`

## /sync/syncpoints

Replies with a list of syncpoints present on this instance 

*Params*
- None

*Returns*

Jsonified list of `changelog::SyncPoint`


## /sync/file_details

Replies with details about the specified file

*Params*
 - `file_id`: Integer ID of the file in the database

*Returns*
Jsonified `foreign_server::FileDetails`


## /sync/file

Returns the raw data of the specified file

*Params*
 - `file_id`: Integer ID of the file in the database

*Returns*
Raw byte content of the file

## /sync/thumbnail

Returns the raw data of the specified file's thumbnail

*Params*
 - `file_id`: Integer ID of the file in the database

*Returns*
Raw byte content of the file's thumbnail


## /sync/changes

Returns all changes after the specified timestamp.

*Params*
 - `starting_timestamp`: The timestamp which all changes returned should be later than

*Returns*
Jsonified `Vec<changelog::Change>`


## /sync/apply_changes

Apply a list of changes to this instance by fetching file details from the foreign
server from which this request arrived. The foreign server can be reached on the specified
port

*Params*
 - `port` The port where the foreign server that initialized the request can be reached

*Body*
The list of changes that should be applied

*Returns*
An empty string


# /subdirectories
Replies with a list of subdirectories of `$FILE_READ_PATH`

*Params*
- None

*Returns*
Jsonified list of filenames


# /ping

Replies with `"pong"`. Used to check if flash has started, specifically by test
scripts

*Params*
- None

*Returns*
`"pong"`


