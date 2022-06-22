# Commands and options

```{eval-rst}
.. include:: ../man_page_fragments/webaves.rst
```

## Log filter syntax

```text
target[span{field=value}]=level
```

The log filter syntax is provided by [Tracing EnvFilter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).
