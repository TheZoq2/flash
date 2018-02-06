# Endpoints

- `/` Serves static files for the frontend
- `/list` Handles any requests that deal with fille lists
- `/album/image` Serves raw images
- `/search` Handles search queries
- `/file_list` *Duplicate of `/list`* Should be removed
- `/sync`

## List

### Actions:

#### Global
Params:
- None

Variants:
- `lists`
#### Per list
Params:
- `list_id`: `usize`. Id of the queried list

Variants:
- `list_info`
- `list_last_saved_index`

#### Per file
Params:
- `list_id`: `usize`. Id of the queried list
- `index`: `usize`. Index of the file in the list

Variantst:
- `get_data`
- `get_file`
- `get_filename`
- `get_thumbnail`
- `save`


#### `lists`

Responds with a list of all `file_lists`

#### `list_info`

Responds with information about a list. The response is the json encoding of
`file_list_response::ListResponse`

#### `list_last_saved_index`

Responds with the index of the last file in the `file_list` which was saved
to the database

#### `get_data`

Responds with a `FileData` struct about the quereied file which contains the path
of the file, the path of the thumbnail as well as the tags of the file

#### `get_file`

Responds with the bytes of the actual file

#### `get_filename`

Responds with the filename of the file

#### `get_thumbnail`

Responds with the raw bytes of the thumbnail

#### `save`

Saves the specified file in the database with the sent tags. Creates a change
at the current time. If the file already exists in the database, it will not
be overwritten but the tags will be updated

Params:
- `tags` List of strings to use as the tags for the saved file


