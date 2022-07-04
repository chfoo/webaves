Synopsis
========

**checksum** [**-h**\ \|\ **--help**] [**-o**\ \|\ **--output**]
[**--overwrite**] <*input*>

Description
===========

Verifies WARC record checksums.

This processes each WARC record for a WARC-Block-Digest field. If the
record includes this field, the checksum is computed for the records
block.

The output is formatted as the records ID, a space, and one of ok, fail,
or skip.

Options
=======

**-h**, **--help**
   Print help information

**-o**, **--output** [default: -]
   Path to output file

**--overwrite** [default: false]
   Allow overwriting existing files

<*input*>
   Path to WARC file
