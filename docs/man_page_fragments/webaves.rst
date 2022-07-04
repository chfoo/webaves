Synopsis
========

**webaves** [**-h**\ \|\ **--help**] [**-V**\ \|\ **--version**]
[**-l**\ \|\ **--log-level**] [**--verbose**] [**--log-filter**]
[**--log-file**] [**--log-format**] <*subcommands*>

Description
===========

Web archive software suite

Options
=======

**-h**, **--help**
   Print help information

**-V**, **--version**
   Print version information

**-l**, **--log-level** [default: warn]
   Set the level of severity of logging messages

**--verbose** [default: false]
   Print informative messages such as progress bars or status updates.
   The log level is also adjusted to "info" if not set.

**--log-filter**
   Filter level of severity and targets of logging messages

**--log-file**
   Write logging messages to a file

**--log-format** [default: default]
   Format of logging messages.

By default, logging output is formatted for human consumption. For
processing, JSON formatted output can be specified instead.

Subcommands
===========

dns-lookup
   Lookup DNS records

warc
   Process WARC files

help
   Print this message or the help of the given subcommand(s)

Version
=======

v0.0.0
