# Webaves

Web archiving software suite.

**Work in progress.** The code, functionality, and documentation is incomplete.

What's mostly implemented:

* WARC files
  * List contents, dump & load as JSON, and extract files
  * API: read and write WARC files

The following is planned:

* Traditional web crawler & archiver
* MITM proxy server capture
* Browser-based capture
* Alternative archive file format

## Installation

Downloads can be found on the [Releases section](https://github.com/chfoo/webaves/releases).

If you want to compile the application yourself, you can do so using cargo from the [`webaves-app` crate](crates/webaves-app/README.md).

## Usage

For information on how to use the application, see the [user guide](https://webaves.readthedocs.io/). If you need help, please check the [Discussions](https://github.com/chfoo/webaves/discussions) section.

 [![Documentation Status](https://readthedocs.org/projects/webaves/badge/?version=latest)](https://webaves.readthedocs.io/en/latest/?badge=latest)

## Developers

The components of Webaves can be reused in your own Rust projects from the [`webaves` crate](crates/webaves/README.md).

![Crates.io](https://img.shields.io/crates/v/webaves) ![docs.rs](https://img.shields.io/docsrs/webaves)

## Contributing

See [Contributing](CONTRIBUTING.md) for information about bug reports and contributing to the project.

[List of sponsors](sponsors.md)

## License

Copyright 2022 Christopher Foo. Licensed under Mozilla Public License Version 2.0.
