program-about = Web archive software suite

input-warc-file-help = Path to WARC file
input-json-file-help = Path to JSON file
output-file-help = Path to output file
output-dir-help = Path of directory to write files
output-compression-format-help = Apply compression to the output
output-as-json-help = Format the output as JSON
output-warc-file-help = Path to output WARC file
allow-overwrite-help = Allow overwriting existing files

verbose-help = Print informative messages
verbose-help-long = Print informative messages such as progress bars or status updates. The log level is also adjusted to "info" if not set.

log-level-help = Set the level of severity of logging messages
log-filter-help = Filter level of severity and targets of logging messages
log-file-help = Write logging messages to a file
log-format-help = Format of logging messages
log-format-help-long =
    Format of logging messages.

    By default, logging output is formatted for human consumption. For processing, JSON formatted output can be specified instead.

doh-server-help = Address and hostname of DNS-over-HTTPS server
doh-server-help-long =
    Address and hostname of DNS-over-HTTPS server.

    Example: "10.0.0.0:443/dns.example.com" specifies IP address 10.0.0.0, port number 443, and a hostname of dns.example.com.

dns-lookup-about = Lookup DNS IP addresses and records
dns-lookup-about-long =
    Lookup DNS IP addresses and records.

    This command can be used to diagnose DNS resolver issues. It does not use the operating system's resolver but contacts servers on the internet directly.

dns-lookup-address-about = Lookup IP addresses for a hostname
dns-lookup-record-about = Lookup records for a hostname
dns-lookup-hostname-help = Target hostname to query
dns-lookup-record-type-help = DNS record type as string or integer

warc-about = Process WARC files
warc-about-long = Read, manipulate, or write WARC files and records
warc-dump-about = Transform WARC files to JSON formatted output
warc-list-about = Listing of file contents using header fields
warc-load-about = Transform JSON formatted input to WARC file
warc-pack-about = Repackages WARC files
warc-pack-about-long = Repackages WARC files by splitting or joining them.

    This command can be used to recompress, split, and join WARC files.

    Although it is safe to concatenate WARC files without the use of a WARC aware tool, recompression and splitting is not. When using compression, each record should be individually compressed (multistream). When splitting files, WARC consuming software may expect records such "warcinfo" to be first or "request" and "response" records to be in the same file. This command will attempt to automatically handle them.
warc-extract-about = Decode and extract documents to files
warc-extract-about-long = Decode and extract documents to files.

    This command will attempt to decode and extract as many documents as possible from response and resource records. By default, the files will be placed in directories similar to its original URL.

    This command does *not* recreate a website for local browsing; this command is intended for use as an "unzipping" tool.
warc-checksum-about = Verifies checksums
warc-checksum-about-long = Verifies WARC record checksums.

    This processes each WARC record for a "WARC-Block-Digest" field. If the record includes this field, the checksum is computed for the record's block.

    The output is formatted as the record's ID, a space, and one of "ok", "fail", or "skip".

warc-list-show-field-with-name-help = Show values with the given field name
warc-list-include-file-help = Include filename and file position
