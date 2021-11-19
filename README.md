# Cyclos Protocol v2

Faithful port of Uniswap v3

## Resources

- Account diagram and library tree: https://drive.google.com/file/d/1S8LMa22uxBh7XGNMUzp-DDhVhE-G9S2s/view?usp=sharing

- Task tracker: https://github.com/orgs/cyclos-io/projects/1

# Max values

| Variable                                         | Type        | Min                                       | Max (inclusive)                                             | Rationale                                                                                                                                                                                                         |
| ------------------------------------------------ | ----------- | ----------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Liquidity, liquidity\_net, liquidity\_gross      | u32         | u32::MIN = 0                              | u32::MAX = 2^32 - 1                                         | Token quantities should be u64. x = L/√P and y = L√P. L and √P must be<br>u32 so that token quantities don't overflow u64.                                                                                        |
| bitmap                                           | \[u128; 2\] |                                           |                                                             | To store 256 bit bitmap. bitmaps crate converts \[u128;2\] into this type                                                                                                                                         |
| fee                                              | u32         | 0                                         | 1000000 (exclusive), i.e. 999,999                           | 1000000 means 100% fee. 1 unit represents 0.0001% = 0.000001.<br>Uniswap uses u24, but we'll have to use u32 in rust.                                                                                             |
| fee growth                                       | f64         |                                           |                                                             | Free growth per unit of liquidity, which will be fractional (Uni uses X128 to<br>represent fraction).<br>Practically fee growth will not be high (it's per unit), but f64 allows us to store<br>smaller fractions |
| sqrt\_price √P.                                  | f64         | 1.0001^(-443636/2)<br>\= 2.32835 × 10^-10 | u32::MAX as float = 2^32 - 1<br>\= 4294967295 = 4.29 x 10^9 |                                                                                                                                                                                                                   |
| tick                                             | i32         | \-443636                                  | floor (log (√1.0001) (4294967295)) = 443636                 | Uniswap uses i24, but this type is unsupported in Rust. Ensure value doesn't<br>exceed limits.<br>Take min tick as negative of max (taken for granted)                                                            |
| tick\_spacing                                    | u16         | 0                                         | 16383                                                       | Taken from Uniswap. Uniswap stored as i24 to remove type conversions, but<br>we can use u16 to save space.                                                                                                        |
| tokens\_owed\_0, tokens\_owed\_1, protocol\_fees | u64         |                                           |                                                             | u64 token quantity is set by SPL token program. Divide by decimal places<br>when displaying to user                                                                                                               |
# Unresolved questions

1. How to replicate Uniswap's rounding? We're using floats which give results directly?
    - No point of rounding up where floats are returned.
    - Rounding up can be done when finding integers. +1 if modulo is not zero. Can use to find fees in [SwapMath.sol](https://github.com/Uniswap/v3-core/blob/f03155670ec1667406b83a539e23dcccf32a03bc/contracts/libraries/SwapMath.sol) and [Pool.sol](https://github.com/Uniswap/v3-core/blob/234f27b9bc745eee37491802aa37a0202649e344/contracts/UniswapV3Pool.sol)

2. Should we avoid floats?
    - Reasons to avoid
        1. Cannot round up / down
        2. Higher compute cost
        3. div mod ?
    - Reasons to use
        1. Ready function to find log to base
        2. Anchor will support floats
    - Alternatives
        1. Use synthetify's number or [fixed crate](https://github.com/Synthetify/synthetify-protocol/blob/master/programs/exchange/src/decimal.rs)
        2. Will need custom formula for log
            - Change of base rule: from log2 to log1.0001
            - https://stackoverflow.com/questions/3272424/compute-fast-log-base-2-ceiling


# U128.128 precision loss issue

To preserve bits

1. Get `1/0.757 * 2^128` in decimal format. Get all integer bits and discard decimal part. This is the value stored in the U128.128 variable.

    ```
    1/0.757 * 2^128 = 49514355245625447111459190794938192147.95
    ```

3. Round up (empirically confirmed): `49514355245625447111459190794938192148`


2. Now convert into hex. `449514355245625447111459190794938192148 as hex`. This will preserve bits.

    ```
    449514355245625447111459190794938192148 = 1522d50d305bfbf11ec62bf687f279114_16
    ```
## Values

```
2^128 / 1.0001^(2^(i - 1)) for i in [0, 20)
```

Rounding principles:
1. Round down if less than 0.5
2. Round up if greater than or equal to 0.5

```
k0 = 2^128 / 1.0001^(2^(0 - 1)) = 340265354078544963557816517032075149313.449 = 340265354078544963557816517032075149313 (round down) = 0xfffcb933bd6fad37aa2d162d1a594001_16 (perfect match)

k1 = 2^128 / 1.0001^(2^(1 - 1)) = 340248342086729790484326174814286782777.722 = 340248342086729790484326174814286782778 (round up) = 0xfff97272373d413259a46990580e213a_16 (perfect match)

k2 = 2^128 / 1.0001^(2^(2 - 1)) = 340214320654664324051920982716015181259.59 = 340214320654664324051920982716015181260 (round up) = fff2e50f5f656932ef12357cf3c7fdcc_16 (perfect match)
```

## For Solana

- Calculate as a
```
2^128 / 1.0001^(2^(i - 1)) for i in [0, 20)
```