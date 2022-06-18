# `warc` subcommand

The `warc` subcommand provides manipulation of WARC files.

For a list of subcommands, type:

    webaves warc --help

## `dump`

The `dump` subcommand reads WARC files and transforms them to JSON formatted output.

Example:

    webaves --verbose warc dump input_file.warc.gz output_file.json

The output format is multiple JSON documents where each document is on a single line.

For each record in the WARC file, it outputs 3 types of documents:

1. `{"Header": ... }`: Header portion of the record.
2. Multiple `{"Block": {"data": [..]}}`: Block portion of the record.
3. `"EndOfRecord"`: Indicates the end of a record.
