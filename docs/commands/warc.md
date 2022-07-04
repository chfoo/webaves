# warc subcommand

## Overview

```{eval-rst}
.. include:: ../man_page_fragments/webaves.warc.rst
```

## checksum

```{eval-rst}
.. include:: ../man_page_fragments/webaves.warc.checksum.rst
```

### Example

```bash
webaves --verbose warc checksum input_file.warc.gz
```

## dump

```{eval-rst}
.. include:: ../man_page_fragments/webaves.warc.dump.rst
```

### Example

```bash
webaves --verbose warc dump input_file.warc.gz --output output_file.json
```

### Format

The output format is multiple JSON documents where each document is on a single line.

For each record in the WARC file, it outputs 3 types of documents:

1. Header portion of the record.
2. Multiple block portions of the record.
3. End of a record indicator.

Example header:

```json
{
    "Header": {
        "version": "WARC/1.1",
        "fields": [
            {
                "name": {
                    "text": "Field-Name"
                },
                "value": {
                    "text": "Field value"
                }
            }
        ]
    }
}
```

Example part of a block:

```json
{
    "Block": {
        "data": [ 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33 ]
    }
}
```

End of record indicator:

```json
"EndOfRecord"
```

## extract

```{eval-rst}
.. include:: ../man_page_fragments/webaves.warc.extract.rst
```

### Example

```bash
webaves --verbose warc extract input_file.warc.gz --output docs_output/
```

## list

```{eval-rst}
.. include:: ../man_page_fragments/webaves.warc.list.rst
```

### Example

```bash
webaves warc list input_file.warc.gz
```

## load

```{eval-rst}
.. include:: ../man_page_fragments/webaves.warc.load.rst
```

### Example

```bash
webaves warc --verbose load input_file.json --output output_file.warc.zstd --format zstd
```
