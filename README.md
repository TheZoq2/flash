# Flash

![Screenshot of main interface](screenshot.png)

This is a photo management program written in rust. It is designed to let you quickly go
through new photos to pick out the best ones, and to allow you to easily find old photos
that you have taken. This is done by letting you assign searchable tags to the photos you save.

The program consists of a backend server that is written in rust which serves photos to frontends.
The frontend is written in elm and is served by the rust server.

## Usage

- Install the rust compiler, cargo and postgresql.
- Install diesel-cli using `cargo install diesel-cli`
- Create a database user and temporarily give it superuser priviliges 
`ALTER USER <username> WITH SUPERUSER`
- Run `diesel database setup`
- create a `.env` file containing the following:
    - `DATABASE_URL=postgres://username:password@url/database_name`
    - `FILE_STORAGE_PATH=<path where you want the saved files to be stored>`
    - `FILE_READ_PATH=<A folder where you want to search for new files>`
- Run the server `cargo run`
- Go to localhost:3000/album.html

To search for previously saved photos, search for `of <list of tags>`

To add new photos, search for `/path/to/subfolder/of/FILE_READ_PATH`





