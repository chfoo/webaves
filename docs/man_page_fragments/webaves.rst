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
   Print informative and progress messages

**--log-filter**
   Filter level of severity and targets of logging messages

**--log-file**
   Write logging messages to a file.

**--log-format** [default: default]
   Format of logging messages

Subcommands
===========

dns-lookup
   Lookup DNS records

warc
   Process WARC files.

help
   Print this message or the help of the given subcommand(s)

Version
=======

v0.0.0
