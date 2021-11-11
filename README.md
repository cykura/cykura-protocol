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
            

<!-- ln(a/b) = ln(a) - ln(b)  a = number, b = decimal ->