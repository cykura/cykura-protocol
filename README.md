# Cyclos Protocol v2

Faithful port of Uniswap v3

## Resources

- Account diagram and library tree: https://drive.google.com/file/d/1S8LMa22uxBh7XGNMUzp-DDhVhE-G9S2s/view?usp=sharing

- Task tracker: https://github.com/orgs/cyclos-io/projects/1

# Max values

| Variable             | Type        | MIN                | MAX                                         | Rationale                                                                           |
| -------------------- | ----------- | ------------------ | ------------------------------------------- | ----------------------------------------------------------------------------------- |
| Liquidity            | u32         | u32::MIN = 0       | u32::MAX = 2^32 - 1                         | x = L/√P and y= L√P. To get x and y as u64, L and P should be u32                   |
| sqrt_price           | f64         | 1.0001^(-443636/2) = 
2.32835×10^-10
  | u32::MAX as f64 = 4294967295 = 4.29 x 10^9               | Float needed because it's a square root. Min can't be 0 (needs i=infinity) |
| tick i               | i32         | -443636            | floor (log (√1.0001) (4294967295)) = 443636 | Minimum will be negative of MAX                                                     |
| tick_spacing         | u16         |                    |                                             |                                                                                     |
| liquidity_net        | u32         |                    |                                             |                                                                                     |
| liquidity_gross      | u32         |                    |                                             |                                                                                     |
| fee_growth           | f64         |                    |                                             |                                                                                     |
| protocol_fees        | u64         |                    |                                             |                                                                                     |
| bitmap               | [bool; 256] |                    |                                             |                                                                                     |
| tokens_owed_0, tokens_owed_1        | u64         |                    |                                             |                                                                                     |


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