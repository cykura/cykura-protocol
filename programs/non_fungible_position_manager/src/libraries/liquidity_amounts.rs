/// Implementations of formula 6.29 and 6.30 to find
/// liquidity <> token_0, token_1

// Liquidity received for a given amount of token_0 and price range
// ΔL = Δx (√P_upper x √P_lower)/(√P_upper - √P_lower)
pub fn get_liquidity_for_amount_0(
    mut sqrt_price_a: f64,
    mut sqrt_price_b: f64,
    amount_0: u64,
) -> u32 {
    // sqrt_price_b (upper) should be greater than sqrt_price_a (lower)
    if sqrt_price_a > sqrt_price_b {
        let temp = sqrt_price_a;
        sqrt_price_a = sqrt_price_b;
        sqrt_price_b = temp;
    }

    (amount_0 as f64 * sqrt_price_b * sqrt_price_a / (sqrt_price_b - sqrt_price_a)) as u32
}

// Liquidity received for a given amount of token_1 and price range
// ΔL = Δy / ( √(P_upper) * √(P_lower) )
pub fn get_liquidity_for_amount_1(
    mut sqrt_price_a: f64,
    mut sqrt_price_b: f64,
    amount_1: u64,
) -> u32 {
    // sqrt_price_b (upper) should be greater than sqrt_price_a (lower)
    if sqrt_price_a > sqrt_price_b {
        let temp = sqrt_price_a;
        sqrt_price_a = sqrt_price_b;
        sqrt_price_b = temp;
    }

    (amount_1 as f64 / (sqrt_price_b * sqrt_price_a)).round() as u32
}

// Computes the maximum amount of liquidity received for a given amount
// of token0, token1, the current pool prices and the prices at the tick boundaries
// Formulae 6.29 and 6.30
// @param sqrt_price Square root of current pool price
// @param sqrt_price_a A sqrt price representing first tick boundary
// @param sqrt_price_b A sqrt price representing second tick boundary
// @param amount_0 Amount of token_0 sent in
// @param amount_1 Amount of token_1 sent in
pub fn get_liquidity_for_amounts(
    sqrt_price: f64,
    mut sqrt_price_a: f64,
    mut sqrt_price_b: f64,
    amount_0: u64,
    amount_1: u64,
) -> u32 {
    // sqrt_price_b (upper) should be greater than sqrt_price_a (lower)
    if sqrt_price_a > sqrt_price_b {
        let temp = sqrt_price_a;
        sqrt_price_a = sqrt_price_b;
        sqrt_price_b = temp;
    }
    let liquidity: u32 = if sqrt_price <= sqrt_price_a {
        // If current price is less the or equal P_lower
        // can only supply token_0, not token_1
        get_liquidity_for_amount_0(sqrt_price_a, sqrt_price_b, amount_0)
    } else if sqrt_price < sqrt_price_b {
        // If current price is in within range
        // can supply both tokens
        let liquidity_0 = get_liquidity_for_amount_0(sqrt_price_a, sqrt_price_b, amount_0);
        let liquidity_1 = get_liquidity_for_amount_1(sqrt_price_a, sqrt_price_b, amount_0);

        u32::min(liquidity_0, liquidity_1)
    } else {
        // If current price is greater than P_upper
        // can only supply token_1, not token_0
        get_liquidity_for_amount_1(sqrt_price_a, sqrt_price_b, amount_1)
    };

    liquidity
}

// Computes the amount of token_0 for a given amount of liquidity and a price range
// Δx = ΔL (√P_upper - √P_lower) / (√P_upper x √P_lower)
pub fn get_amount_0_for_liquidity(
    mut sqrt_price_a: f64,
    mut sqrt_price_b: f64,
    liquidity: u32,
) -> u64 {
    // sqrt_price_b (upper) should be greater than sqrt_price_a (lower)
    if sqrt_price_a > sqrt_price_b {
        let temp = sqrt_price_a;
        sqrt_price_a = sqrt_price_b;
        sqrt_price_b = temp;
    }

    liquidity as u64 * (((sqrt_price_b - sqrt_price_a) / (sqrt_price_b * sqrt_price_a)) as u64)
}

// Computes the amount of token_1 for a given amount of liquidity and a price range
// Δy = ΔL ( √(P_upper) - √(P_lower) )
pub fn get_amount_1_for_liquidity(
    mut sqrt_price_a: f64,
    mut sqrt_price_b: f64,
    liquidity: u32
) -> u64 {
    // sqrt_price_b (upper) should be greater than sqrt_price_a (lower)
    if sqrt_price_a > sqrt_price_b {
        let temp = sqrt_price_a;
        sqrt_price_a = sqrt_price_b;
        sqrt_price_b = temp;
    }

    liquidity as u64 * ((sqrt_price_b - sqrt_price_a) as u64)
}

// Get amounts of token_0 and token_1 for a given amount of liquidity,
// current pool price and prices at tick boundaries
// @param sqrt_price sqrt of current pool price
// @param sqrt_price_a A sqrt price representing first tick boundary
// @param sqrt_price_b A sqrt price representing second tick boundary
// @param liquidity The liquidity being balued
pub fn get_amounts_for_liquidity(
    sqrt_price: f64,
    mut sqrt_price_a: f64,
    mut sqrt_price_b: f64,
    liquidity: u32,
) -> (u64, u64) {
    // sqrt_price_b (upper) should be greater than sqrt_price_a (lower)
    if sqrt_price_a > sqrt_price_b {
        let temp = sqrt_price_a;
        sqrt_price_a = sqrt_price_b;
        sqrt_price_b = temp;
    }
    let mut amount_0 = 0;
    let mut amount_1 = 0;

    if sqrt_price <= sqrt_price_a {
        // entire liquidity is in token_0
        amount_0 = get_amount_0_for_liquidity(sqrt_price_a, sqrt_price_b, liquidity);
    } else if sqrt_price < sqrt_price_b {
        // liquidity is in a mix of token_0 and token_1
        amount_0 = get_amount_0_for_liquidity(sqrt_price, sqrt_price_b, liquidity);
        amount_1 = get_amount_1_for_liquidity(sqrt_price_a, sqrt_price, liquidity);
    } else {
        // entire liquidity is in token_1
        amount_1  = get_amount_1_for_liquidity(sqrt_price_a, sqrt_price, liquidity);
    }

    (amount_0, amount_1)
}
