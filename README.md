<p align="center">
  <h3 align="center">Platzhalter</h3>
  <p align="center">A service to generate placeholder images</p>
</p>

# About

There are often situations in web development where you need cheap throwaway placeholder images.
Because it's expensive and tedious to manually save an image and place it in your markup (html e.g.),
services like [Placeholder.com](https://placeholder.com/) generate images on the fly via a mix of http
query and path params.

# Why self hosted?

You can be definitely sure that the service is delivering images and nothing else which is especially
important in corporate settings. And of course you can customize the output and e.g. avoid watermarks
in our output.

# Why placeholder?

A lot of services that I've used and tested before were incredibly slow in serving a request. Some services
took up to **1 second** to generate a simple 450x450 pixel image. That's why I've created placeholder,
a fast and robust implementation in Rust. Once an image is generated it gets cached via [sled](https://github.com/spacejam/sled)
which is an incredibly fast embedded database. Of course, caching binaries in memory could easily become
a scaling issue but that's up to the user to decide.

# Disclaimer

The project is still WIP and won't receive many updates in the future especially to improve the ease of deployment.
I for my part simply compile the project and rsync it to my server.

