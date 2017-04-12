# Sample searches

At night could be a tag or 8 < time < 8
List of tags
`images of houses, cars taken at night`

Last year implies a time +- a time interval
`containing dogs from last year`

A time can be a keyword or a number
`taken in 2012`

A keyword can also be a 'number
`Taken in july`

An interval of time based on another date. Again, the last keyword
`in the last 6 months`

Times can also be lists
`Taken in july or august`

Lists can either be and or or
`images of houses and cars`

Is this allowed?
`images of houses or cars`


Should these be counted as and or or?
`images houses, cars`

Does this one make sense?
`taken in spring, summer`

We can also have locations
`In linköping`
Implied radius?
`around linköping`
Explicit radius
`within 10km of linköping`
Multiple keywords for in or aroudn
`in 10 km of linköping`
Different radiuses depending on the size of the location
`in östergötland`

In can mean different things
`in linköping in 2012`


# Language constructs

## Datatypes
- tag: a tag, this could be any string, it might clash with some times or locations "night"
- time: something describing a combination years, months, (weeks?), days, hours etc.
- location: Something describing a physical location in the world. 
A town, a country. Could go along with a radius



## List constructs
```
LISTABLE =
    tag | time | location

LISTING_KEYWORD =
    and | or

LIST<Listable: LIST_TYPE, Keyword: LISTING_KEYWORD> =
    [Listable, .. [[Keyword] Listable]
```

## Function keywords
```
TAG_LIST_KEYWORD =
    of | containing | with

TAG_LIST =
    TAG_LIST_KEYWORD LIST<tag, and|or>
```


## Time keywords
```
CURRENT_TIME =
    today

TIME_SPECIFIER 
    = year(s)
    | month(s)
    | week(s)
    | day(s)
    | hour(s)
    ...

TIME_AMOUNT
    = Int TIME_SPECIFIER

TIME_MODIFIER 
    = last
    | past

YEAR
    = Int (> 1000)

MONTH
    = january
    | february
    | ...
    | december

SEASON
    = summer
    | winter
    | autumn
    | fall
    | spring

TIME_OF_DAY
    = morning
    | night

TIME_POINT
    = YEAR
    | MONTH
    | SEASON
    | TIME_OF_DAY

TIME_INTERVAL
    = TIME_MODIFIER TIME_AMOUNT
    | TIME_POINT

TIME_LIST
    List
```
