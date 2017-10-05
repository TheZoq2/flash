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


### Examples
- Moudulus times
- *Prefix*: `this`
    - today (this day)
    - week
    - month
    - week
    - year

- Relative times
- *Prefix*: `the past`
    - the past day
    - the past month
    ...

- Pattern like
- *Prefix* `matching`
    - July
    - 2017
    - July 2017
    - 20th july

