# Benchmarks

## Table of Contents

- [Benchmark Results](#benchmark-results)
    - [add](#add)

## Benchmark Results

### add

|            | `rustref (indirect)`          | `rustref (direct)`              | `native`                        | `memory`                         |
|:-----------|:------------------------------|:--------------------------------|:--------------------------------|:-------------------------------- |
| **`i128`** | `60.69 ns` (✅ **1.00x**)      | `58.52 ns` (✅ **1.04x faster**) | `61.35 ns` (✅ **1.01x slower**) | `96.64 ns` (❌ *1.59x slower*)    |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)

