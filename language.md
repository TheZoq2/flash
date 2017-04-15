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
DATE
    = YearNumber Month Day

TIME_TYPE
    = year
    | month
    | week
    | day

SINGLE 
    = today
    | this TIME_TYPE

INTERVAL
    = between DATE and DATE
    | past TIME_TYPE
    | last TIME_TYPE
```

### Examples
- From today: `today`
- From this month: `this month`
- From the last 30 days `past month`
- From last year `last year`
- From the last 365 days `past year`
- From a specific month `in july`




