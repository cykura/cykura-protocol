# Cyclos Protocol v2

Concentrated liquidity on Solana

## Resources

- Account diagram and library tree: https://drive.google.com/file/d/1S8LMa22uxBh7XGNMUzp-DDhVhE-G9S2s/view?usp=sharing

- Task tracker: https://github.com/orgs/cyclos-io/projects/1

## Oracle design

- We emulate Uniswap's observation array using program derived accounts. Each PDA is seeded with an **index** representing it's position in the array. The seeds are

```
[OBSERVATION_SEED, token_0, token_1, fee, index]
```

- Oracle observations are updated when
    1. [A swap is performed](./programs/core/src/lib.rs#L1483)
    2. [A position is modified](./programs/core/src/lib.rs#L2387)

- Uniswap performs checkpointing every time the pool is touched **for the first time in a block**. With a max limit of 65k observations Uniswap can store observations for 9 days. Solana has a block time of 0.5s compared to 14s in Ethrereum.

- To replicate Uniswap's 9 day oracle in Solana
    1. block time is partitioned in 14 second intervals starting with epoch time 0.
    ```
    |----partition_0 [0, 14)----|----partition_1 [14, 28)----|----partition_0 [28, 42)----|
    ```

    [The partition start time is found as](./programs/core/src/lib.rs#L1483)

    ```rs
    let next_observation_start_time = (latest_observation.block_timestamp / 14 + 1) * 14;
    ```

    2. The very first observation is written at slot 0.

    3. Solana's clock tracks seconds despite its block time being 0.5 seconds. Checkpointing is performed every time the pool is touched **for the first time in a second**.

    4. If the pool was touched in the same partition as the last saved observation, overwrite it. Else write on to the next slot.

- Observations in Uniswap are written from index 0 and onwards till we reach the end index, after which observations are wrapped around from start like a **circular buffer**. [The current observation is tracked with an `observationIndex` variable stored at the pool level](https://github.com/Uniswap/v3-core/blob/ed88be38ab2032d82bf10ac6f8d03aa631889d48/contracts/UniswapV3Pool.sol#L62). We store a similar `observation_index` variable in the [pool account](https://github.com/cyclos-io/cyclos-protocol-v2/blob/ba961915f85afc253ff301a7db258c29b00cac28/programs/core/src/states/pool.rs#L40).

- Uniswap's observation array can be grown to store more observations. It follows these rules when writing to new slots:
    1. Newly created slots are [marked as uninitialized](https://github.com/Uniswap/v3-core/blob/ed88be38ab2032d82bf10ac6f8d03aa631889d48/contracts/libraries/Oracle.sol#L117). A placeholder timestamp value of 1 is stored to perform an SSTORE and pay for slot creation.
    2. The pool variable `observationCardinality` stores the number of **initialized slots**, and `observationCardinalityNext` stores the count of **created slots**. `observationCardinalityNext` is incremented on slot creation, but not `observationCardinality`.
    3. When we reach the end element allowed by `observationCardinality`, it's value is incremented. This repeats for future observations until `observationCardinality` becomes equal to  `observationCardinalityNext`.