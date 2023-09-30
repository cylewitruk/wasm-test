# Benchmarks

## Table of Contents

- [Benchmark Results](#benchmark-results)
    - [add](#add)

## Benchmark Results

### add

|            | `compiled wat`           | `extref`                         | `rustref (indirect)`            | `rustref (stack)`               | `rustref (direct)`              | `native`                        | `memory`                         |
|:-----------|:-------------------------|:---------------------------------|:--------------------------------|:--------------------------------|:--------------------------------|:--------------------------------|:-------------------------------- |
| **`i128`** | `57.10 ns` (✅ **1.00x**) | `236.10 ns` (❌ *4.14x slower*)   | `62.76 ns` (✅ **1.10x slower**) | `50.55 ns` (✅ **1.13x faster**) | `57.81 ns` (✅ **1.01x slower**) | `65.60 ns` (❌ *1.15x slower*)   | `95.17 ns` (❌ *1.67x slower*)    |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)

