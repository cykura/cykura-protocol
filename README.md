# Cyclos Protocol v2

Concentrated liquidity on Solana

## Resources

- Account diagram and library tree: https://drive.google.com/file/d/1S8LMa22uxBh7XGNMUzp-DDhVhE-G9S2s/view?usp=sharing

- Task tracker: https://github.com/orgs/cyclos-io/projects/1

## Oracle design

- Cyclos adapts Uniswap's circular observation array to Sealevel using program derived accounts (PDA). Each PDA is seeded with an **index** representing its array position in the given way-

```
[OBSERVATION_SEED, token_0, token_1, fee, index]
```

- Index is incremented for every successive observation and wrapped around at the end element. For a cardinality 3, the indexes will be `0, 1, 2, 0, 1` and so on. The index for the latest position is found as `pool.observation_index % cardinality`.

- Cardinality can be grown to store more observations.
    1. Created slots are [marked as uninitialized](https://github.com/Uniswap/v3-core/blob/ed88be38ab2032d82bf10ac6f8d03aa631889d48/contracts/libraries/Oracle.sol#L117). A placeholder timestamp value of 1 is stored to perform an SSTORE and pay for slot creation. Cyclos analogously creates a program account.
    2. The pool variable `observationCardinality` stores the number of **initialized slots**, and `observationCardinalityNext` stores the count of **created slots**. `observationCardinalityNext` is incremented on slot creation, but not `observationCardinality`.
    3. When we reach the end element allowed by `observationCardinality`, the value of this variable is incremented so that the next uninitialized slot can be reached for writing the next observation. This repeats until every uninitialized slot is filled.

- Obervations are updated on
    1. [Swaps](./programs/core/src/lib.rs#L1483)
    2. [Position modifications, i.e. creating, removing and modifying positions](./programs/core/src/lib.rs#L2387)

- Uniswap checkpoints data whenever a pool is touched **for the first time in a block**. Other interactions within the block are not recorded.

- Uniswap's observation array can store 65k slots, equivalent to 9 days of recordings given Ethereum's 14 second block time. 65k slots would result in just a day's worth of readings on Solana given its 0.5 second block time. We introduce a time partitioning mechanism to overcome this limitation

    1. Block time is partitioned in 14 second intervals starting with epoch time 0.
    ```
    |----partition_0 [0, 14)----|----partition_1 [14, 28)----|----partition_0 [28, 42)----|
    ```

    2. To know the partition for a timestamp, perform floor division by 14.

    ```
    partition = floor(timestamp / 14)
    ```

    3. Find the partitions for the current block time (partition_current) and for the last saved observation (partition_last).

    4. If `partition_current > partition_last` checkpoint in the next slot. Else overwrite the current slot with new data.

- Unlike EVM, the last and next observation accounts must be found on client side in Sealevel.
    1. Last observation state: Acccount storing the last checkpoint.
    2. Next observation state: The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`. This account is read/modfified only if the next and last checkpoint fall in different partitions. This field can be made optional in future by using remaining accounts.
