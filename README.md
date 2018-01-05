# Flash

![Screenshot of main interface](screenshot.png)

[Demo video showing an example of how to use the program](https://youtu.be/Vxh0yE62RCo)

Flash is a photo management program written in rust. It is designed to let you quickly go
through new photos to pick out the best ones, and to allow you to easily find old photos
that you have taken. This is done by letting you assign searchable tags to the photos you save.

The program consists of a backend server that is written in rust which serves photos to frontends.
At the moment, there are two frontends for the project: [Flash-frontend](https://gitlab.com/TheZoq2/Flash-Frontend) and [Flash-cli](https://gitlab.com/TheZoq2/flash-cli)

Flash frontend is the main interface for the program which allows you to import and view pictures
as well as add tags to them. It is written in Elm and runs in the web browser.

Flash-cli, as the name implies is a commandline interface for flash. It allows you to search
for pictures in the database just like you would in the web-interface but instead of just
viewing them, it creates symlinks to the resulting images. This allows you to directly access
the files if you want to send them to someone, or modify them using external programs.

## Usage

The frontend consists of 2 pages: the album page and the viewer page. On the album page,
you can search for photos by combining the following querys:

- `of things, not stuff and flowers` Searches for pictures containing both things and flowers, but not stuff
- `this day`, `this month` etc. Searches for images taken in the current time unit. For example: `this month` on
august 28 will return all pictures taken in august this year.
- `the past day`, `the past month` etc. searches for images taken in the past 24 hours or 30 days.
- `/path/to/folder` Shows all photos in `folder`. The path to a folder relative to `FILE_READ_PATH`.
This is used for adding new photos into the system

## Installing

- Install the rust compiler, cargo and postgresql.
- Install diesel-cli using `cargo install diesel-cli`
- Create a database user and temporarily give it superuser priviliges 
`ALTER USER <username> WITH SUPERUSER`
- create a `.env` file containing the following:
    - `DATABASE_URL=postgres://username:password@url/database_name`
    - `FILE_STORAGE_PATH=<path where you want the saved files to be stored>`
    - `FILE_READ_PATH=<A folder where you want to search for new files>`
- Run `diesel database setup`
- Compile the frontend
    - `git submodule --recursive init && git submodule --recursive update`
    - `cd frontend`
    - `make`
- Run the server `cargo run`
- Go to localhost:3000/album.html

## Future plans

The files are currently stored on a server in their original format. This works when
the server and the client are on the same computer or network but might be too
slow for mobile connections. A decentralised system where files are cached on each
device might be a good idea.

