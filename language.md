# Sample searches

Searches contain can contain a combination of 3 sets of data. A list of tags to 
include, a timespan when the picture was taken and a geographic region in which
it was taken.

Each of the sets should be preceded by a word that can, in most cases, be used to tell
what kind of data is being specified. Each kind of data can only appear
once in each query

Ideally the sentence should look like `Pictures <query>`, ie. `Pictures of dogs from this year`


# Constructs

## Tags
### Specification
```
TAG
    = [not] String

TAG_LIST
    = [TAG, ... [[and] TAG]]
```

### Examples
quadcopter
quadcopter, 3d printer
quadcopter and 3d printer


## Dates
```
SINGLE 
    = today
    | this year
    | 
```

### Examples
- Pictures from today: `today`
- Pictures from this month: `this month`
- Pictures from the last 30 days `past month`
- Pictures from last year `last year`
- Pictures from the last 365 days `past year`
- Pictures taken in a specific month `in july`




