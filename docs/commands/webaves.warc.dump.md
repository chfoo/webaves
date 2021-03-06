# webaves warc dump

```{eval-rst}
.. include:: fragments/webaves.warc.dump.rst
```

## Example

```bash
webaves --verbose warc dump input_file.warc.gz --output output_file.json
```

## Format

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
