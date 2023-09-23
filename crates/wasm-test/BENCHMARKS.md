# Benchmarks

## Table of Contents

- [Benchmark Results](#benchmark-results)
    - [add](#add)

## Benchmark Results

### add

|            | `rustref (indirect)`          | `rustref (direct)`              | `native`                        | `memory`                          |
|:-----------|:------------------------------|:--------------------------------|:--------------------------------|:--------------------------------- |
| **`i128`** | `62.60 ns` (✅ **1.00x**)      | `61.68 ns` (✅ **1.01x faster**) | `60.75 ns` (✅ **1.03x faster**) | `104.45 ns` (❌ *1.67x slower*)    |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)

