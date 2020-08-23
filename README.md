# Flounder!

An ultra-lightweight Gemini web portal. View online at https://flounder.online/

I'm really interested in the ideas behind [Tildeverse](https://tildeverse.org/) and the [IndieWeb](https://indieweb.org/), and wanted to make an application that takes some of the ideas behind these communities and makes a radically simple and easy-to-use version of it. [Neocities](https://neocities.org/) was another big inspiration: giving users the power and transparency of directly editing HTML, while making the process accessible and easy to learn. Gemini is such a straightforward protocol that making a static website becomes even easier!

Maybe Future Feature Ideas:

* Support [Webmentions](https://indieweb.org/Webmention)?

## Development

Requires sqlite3 libraries

To run the development server:

```bash
# create a config file (you're gonna want to change the default values)
$ cp example_config.toml config.toml
$ cargo build .
# Apply the schema
$ sqlite3 < schema.sql db/sqlite3.db
# Run development server
$ ./target/debug/flounder run
```

## Deployment

Flounder's ultimate goal is to be able to be easily self-hosted. If you're careful and know what you're doing, you could ry deploying it, but I haven't written any guides on how to do so yet. The Actix server is not secure as-is, I use Nginx to handle a lot of the proxying. I'll share resources when this is more stable. 

Here's a rough outline of things you'll need to configure your server to do:
- xss protection in production
- tls in production
- block /user (this prevents xss. Images and especially svgs are attack vectors for xss)
- proxy {user}.flounder.online to flounder.online/user/{user} with default path being index.gmi
- proxy {user}.flounder.online/static/style.css to flounder.online/static/style.css
- limit body to 32KB


Flounder requires two servers -- an https server and a gemini server. For my deployment, I use nginx and molly-brown. Much of the routing and settings occurs at this level. I will add this to the documentation once it is more stable. 

I have to use a modified version of molly-brown to route subdomains. I'll share when it's more stable or merged into master.

The admin user is a special user that allows you to make site-wide announcements, etc. 
