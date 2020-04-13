# Helper for organization

All helpers are accessible as subcommands of the tools helper:

```bash
tools papers ...
```

Current helpers:

- `backup`: helper to create backups with de-duplication via hardlinks on
  windows
- `papers`: sort papers and rename them in a consistent way using arxiv meta
  data
- `tags`: an interactive tag browser for a collection of markdown documents


Example calls:

```bash
tools backup --ref D:\backup\2020-03-09 C:\Users\USER D:\backup\2020-04-12
tools tags C:\Users\USER\Notes
```
