Synopsis
========

**dns-lookup** [**-h**\ \|\ **--help**] [**--bind-address**]
[**--doh-server**] <*subcommands*>

Description
===========

Lookup DNS records

Options
=======

**-h**, **--help**
   Print help information

**--bind-address**
   Address of outgoing network interface. (Example: 192.168.1.100:0)

**--doh-server**
   Address and hostname of DNS-over-HTTPS server. (Example:
   10.0.0.0:443/dns.example.com)

Subcommands
===========

address
   Lookup IP addresses for a hostname

record
   Lookup records for a hostname

help
   Print this message or the help of the given subcommand(s)
