# Running the CLI application

To run the CLI version of the application, open a terminal application (such as Windows Terminal, Terminal on MacOS, GNOME Terminal, or Konsole). Type the name of the program (`webaves`, or the full path to the program), followed by any options, and press the enter key to run it.

If you installed the application using `cargo install`, the program name will be `webaves-app`.

The CLI application uses subcommands to run specific functionality. To see help when running the application, add the `--help` option.

## Commands and options

```{eval-rst}
.. include:: man_page_fragments/webaves.rst
```

### Log filter syntax

```text
target[span{field=value}]=level
```

The log filter syntax is provided by [Tracing EnvFilter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).
