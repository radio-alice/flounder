# Flounder!

An ultra-lightweight Gemini web portal.

I'm really interested in the ideas behind [Tildeverse](https://tildeverse.org/) and the [IndieWeb](https://indieweb.org/), and wanted to make an application that takes some of the ideas behind these communities and makes a radically simple and easy-to-use version of it. [Neocities](https://neocities.org/) was another big inspiration: giving users the power and transparency of directly editing HTML, while making the process accessible and easy to learn. Gemini is such a straightforward protocol that making a static website becomes even easier!

Maybe Future Feature Ideas:

* Support [twtxt](https://github.com/buckket/twtxt)
* Support [Webmentions](https://indieweb.org/Webmention)? (seems unmaintained?)
* Allow git repository hosting for Power Users

## Deployment

Flounder accepts user-uploaded files and serves them. This requires a great deal of care! The built-in Actix server is NOT suitable for production. I use Nginx, and I (will) store a config in this repo.

## Influences
