Synopsis
========

**dns-lookup** [**-h**\ \|\ **--help**] [**--bind-address**]
[**--doh-server**] <*subcommands*>

Description
===========

Lookup DNS IP addresses and records.

This command can be used to diagnose DNS resolver issues. It does not
use the operating systems resolver but contacts servers on the internet
directly.

Options
=======

**-h**, **--help**
   Print help information

**--bind-address**
   IP address and port number of the outgoing network interface.

Example: "192.168.1.100:0" specifies the network interface with IP
address 192.168.1.100, and 0 to indicate a default port number.

**--doh-server** [default: 1.1.1.1:443/cloudflare-dns.com,8.8.8.8:443/google.dns]
   Address and hostname of DNS-over-HTTPS server.

Example: "10.0.0.0:443/dns.example.com" specifies IP address 10.0.0.0,
port number 443, and a hostname of dns.example.com.

Subcommands
===========

address
   Lookup IP addresses for a hostname

record
   Lookup records for a hostname

help
   Print this message or the help of the given subcommand(s)
