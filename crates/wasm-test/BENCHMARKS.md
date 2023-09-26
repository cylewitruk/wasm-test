# Benchmarks

## Table of Contents

- [Benchmark Results](#benchmark-results)
    - [fold-add-square](#fold-add-square)
    - [add](#add)

## Benchmark Results

### fold-add-square

|            | `extref`                | `rustref`                         |
|:-----------|:------------------------|:--------------------------------- |
| **`i128`** | `2.03 ms` (✅ **1.00x**) | `883.60 us` (🚀 **2.29x faster**)  |

### add

|            | `compiled wat`           | `extref`                         | `rustref (indirect)`            | `rustref (direct)`              | `native`                        | `memory`                          |
|:-----------|:-------------------------|:---------------------------------|:--------------------------------|:--------------------------------|:--------------------------------|:--------------------------------- |
| **`i128`** | `53.17 ns` (✅ **1.00x**) | `228.18 ns` (❌ *4.29x slower*)   | `61.93 ns` (❌ *1.16x slower*)   | `56.13 ns` (✅ **1.06x slower**) | `62.70 ns` (❌ *1.18x slower*)   | `105.57 ns` (❌ *1.99x slower*)    |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)

