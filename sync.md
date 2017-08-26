# Synchronising file changes

Each time a sync is made, a sync 'object' is created. The sync objects contains a 'unique ID' along
with a timestamp for when that sync was made.  An initial sync object is created when the program is
first run.

When syncing, both computers compare their sync objects until they find the last object a common ID.
All changes after the timestamp of that object on both clients are combined into a list of changes
ordered by creation time. Those changes are applied to both databases and missing files among the
clients are exchanged. All sync objects up to that point are synchronised as well. Finally, the
clients agree on a new sync-id and timestamp and the object is stored in the databsae.


## Problems

### Data races

The database synchronisation must most likely be done in a mutexed enviroment to avoid problems
where the user updates files as the sync happens.

Though perhaps that wont be a problem since those changes theoretically happen after the timestamp
when the sync began.

File exchange can probably be done asynchronously.

### File exchange.

File exchange can probably be done using the normal file request functions used in the rest of the
API. However, I might need to keep track of the origin of all files that have not been synced yet in
case the remote server disconnects.

### Change tracking

The changes between each sync timestamp need to either be tracked or calculated. Calculation is
simpler and more efficient but keeping track of 'remove' operations is tricky. If a tag is removed
in one version and present in another, it might mean that it was added by the latter or removed by
the former. It could also have been removed in both and later re-added in one.

#### Possible solutions:

Track all changes in addition to the current state. This works, and avoids the problem of having to
build a changeset on sync, but requires storage of more data, which can be desyncronised. Sync then
is just a matter of applying all changes made since the common timestamp in chronological order on
both servers.




