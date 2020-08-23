# Notes on Changes
* remove unused config vars
* add root_domain config var and change server_name to be the name of the server, not the domain
* /user -> /person :)
* move /static/styles/styles.css to /static/styles.css
* remove FlounderError in favor of using tide's errors and result type– generally helps make error handling a lot cleaner since every route just returns tide::Result, so you can use all the `?` result handling you want
* switch to sqlx over rustqlite– a little cleaner api and works well with tide + tide sessions
* move escape + gemini2html to a utils crate
* separate lib.rs out into records, routes, templates
* put all the user-session routes in one place– /sign/in, /sign/up, /sign/out
* remove extraneous <br>s– css should prob handle this
* only allow editing for text files
* right now I don't have the error page hooked up, since it wasn't used anywhere I could see, but would prob be a good thing to attach as a body to our error responses eventually
* change `.ok()` to `.unwrap()` in main so we can see why it fails if it does
# Associated notes on how to tinker with this

## Routing
client requests route
-> main.rs sorts it into its associated function in /routes
-> route function processes it and returns redirect (starting process over), error code, or a rendered template

## Database
* almost all database functions are encapsulated as methods of the File or Person structs in /records
* if you need new db functions, adding them as extensions to record types is probably the cleanest way to do it
* db functions are usually called from the route handling functions in /routes
* use `.fetch_one(db).await?` for single queries, `.fetch_all(db).await?` for multiple queries, and `.execute(db).await?` for insertions / deletions
* depending on sql return type use `QueryAs<Self>`, `QueryAs<({arbitrary type},)>` or `Query` (for no return type)
* NOTE: non-self `QueryAs` types must be in a tuple (e.g. `(String,)` or you have to implement FromRow yourself I think?

## Sessions
* set session data with `request.session().insert({your-data's-key}, {your data})`
* get session data with `request.session().get({your-data's-key})`
* convenience function to get current session's user is `request.get_person()` - note: this throws a 403 if not logged in
* if being logged in is optional, use `request.session().get("person")`

## Misc
* use File::get_full_path to easily get sanitized full path from file, user, file_dir strings

## Known issues
* sqlx has an sqlite bug that causes really slow responses on some queries, fix is in an [open PR](https://github.com/launchbadge/sqlx/pull/627)
* tide lacks a multipart/form-data handler, so I had to hack one together. it works fine, but we should transition to the tide implementation when [its done](https://github.com/http-rs/http-types/pull/175)
