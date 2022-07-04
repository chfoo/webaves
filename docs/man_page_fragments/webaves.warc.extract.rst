Synopsis
========

**extract** [**-h**\ \|\ **--help**] <**-o**\ \|\ **--output**>
<*input*>

Description
===========

Decode and extract documents to files.

This command will attempt to decode and extract as many documents as
possible from response and resource records. By default, the files will
be placed in directories similar to its original URL.

This command does \*not\* recreate a website for local browsing; this
command is intended for use as an "unzipping" tool.

Options
=======

**-h**, **--help**
   Print help information

**-o**, **--output**
   Path of directory to write files

<*input*>
   Path to WARC file
