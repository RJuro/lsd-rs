# Example configurations

LSD configuration files (`.lsd`) for the models built into lsd-rs, copied from LSD 8.1's `Example/` folder
(GPL, by Marco Valente, Marcelo C. Pereira and the models' authors). Run them with, e.g.:

```bash
lsd-rs run K+S-original examples/K+S-original/Benchmark.lsd --runs 10
```

Some of these files have `SEED 0`, which LSD 8.1 refuses; lsd-rs runs them with seed 1 unless `--seed` is given.
