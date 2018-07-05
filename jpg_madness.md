# Notes
This is a list of notes about reading metadata from .jpg files and the variuos
pitfalls I have come across. Everything in this file *should* be correct but I
wouldn't be surprised if there are issues.

This was written while trying to patch the rust library immeta which is why it
refers to it occationally.

# One compression, two formats
The reason the immeta library doesn't work for the files comming of my camera
seems to be that jpeg has (atleast) two container formats. **JFIF** and **EXIF**.
JFIF seems to be the "official" container but of course, standards are meant
to be broken so camera manufactures decided to make their own format, EXIF.

The way I understand it is that JPEG is actually just a compression standard
and not actually a file format standard. Both JFIF and EXIF are containers
that encapsulate that compression format. 

However, EXIF is also a standard for storing metadata for both audio and images
which might not be .jpeg files which means that EXIF is more than just a container.


# General JPEG things.
JPEG files seem to be comprised of markers and data. Markers come before data
and tell us what the data is. A marker always starts with `FF` and is followed
by another byte telling us what it marks.


Each JPEG file starts with the `SOI` (start of image) marker (`FFB8`). Which 
means that you can figgure out if a file is a .jpeg file or something else
by reading the first 2 bytes.


# Figuring out what container a file uses
Immediately following the SOI marker should be an `APPn` marker. 
APP stands for application specific marker which is a somewhat weird name in my opinion. 
Luckily for us, to figure out what container the file is, the APPn segment contains 
data about just that.

I mentioned before that `APPn` must start at the beginning of the file. To avoid
conflicts between EXIF and JFIF files, EXIF uses APP1 while JFIF uses APP0. 

```
| Format | Marker | Marker byte |
|--------+--------+-------------|
| JFIF   | APP0   | ffe0        |
| EXIF   | APP1   | ffe1        |
```

The APPn data in both EXIF and JFIF contain an "identifier" string that
contains `"EXIF"` or `"JFIF"` depending on the type. But since both
of them use different appn containers, you can probably just ignore them

Finally, the APP part of the file contains thumbnail data if you want to extract
that.

See https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format#File_format_structure 
for details on what the `APP` section contains in the JFIF format.

# Decoding JFIF metadata
This was already implemented in immeta when I came across it which means that
things written here are more likely to be wrong. 

## Reading the dimensions.
In a JFIF formatted file, the dimensions and some other data are stored under
the `SOF0` marker for files compressed using "baseline DCT" and under the `SOF2`
marker for files compressed using "progressive DCT". I don't know what the 
difference is but luckily, both segments are identical when you are looking
for dimensions.

```
| Compression method | Marker | Marker byte |
|--------------------+--------+-------------|
| Baseline DCT       | SOF0   | ffc0        |
| Progressive DCT    | SOF2   | fc02        |
```

The layout of the SOFn segment can be found at this website:
http://vip.sugovica.hu/Sardi/kepnezo/JPEG%20File%20Layout%20and%20Format.htm


# Decoding EXIF
The APP1 segment contains a bunch of data which we will need later. The first 
thing in the segment is a string saying `"EXIF"` followed by two null bytes
(`0x00`). We can ignore that, unless there is some camera that exports in a
format that is not EXIF but uses APP1.

## Big/little endianness
As it turns out, decoding EXIF is a bit more tricky than decoding JFIF. The first
hurdle you need to get across is to read the endianness of the data. It turns
out that while the rest of the JPEG specification uses big-endian bytes, in the
exif sections of the file, the camera manufacturers can decide between big-endian
and little-endian bytes.

The first 8 bytes after the format string are of what is known as the 
"TIFF header" (yes, TIF is another fileformat. That is somehow embedded in 
JPEG. Why?). 

Anyways, the first two bytes of that file contain what we are looking for,
a specification of wether or not we are reading a big-endian or little
endian file. If they are `0x4949 = "II"`, it is (intel alligned), Little endianed.
If they are `0x4d4d = "MM"`, it is (motorola alligned), big endianed.

## Some "padding"
After the big/little endianness specification comes 2 bytes that we don't need
to care about. They should always be `0x2a00`. However, since we either use
little or big endianness, they can also be `0x002a`. 

## Jumping around the file
The last part of the tiff header contains another piece of very usefull information.
It contains the offset of the first `IFD` block, `IFD0`. IFD blocks are where the
actual EXIF data are stored and aparently they don't have to be stored immeadietly
after each other. The offset here is the amount of bytes it is offset from the
first byte of the TIFF header. The specification is 4 bytes long.





#Sources/more details
Wikipedia links explaining some of the details but missing some others
https://en.wikipedia.org/wiki/JPEG
https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format

A pretty good description of the JFIF metadata format
http://vip.sugovica.hu/Sardi/kepnezo/JPEG%20File%20Layout%20and%20Format.htm

http://www.exif.org/Exif2-2.PDF

Really good description of the EXIF format
https://www.media.mit.edu/pia/Research/deepview/exif.html
