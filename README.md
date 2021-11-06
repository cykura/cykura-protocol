# Cyclos Protocol v2

Faithful port of Uniswap v3

## Resources

- Account diagram and library tree: https://drive.google.com/file/d/1S8LMa22uxBh7XGNMUzp-DDhVhE-G9S2s/view?usp=sharing

- Task tracker: https://github.com/orgs/cyclos-io/projects/1


# Float support in anchor

1. IDL should emit "f64" instead of
```json
"type": {
    "defined": "f64"
}`
```

2. Add f64 in idl.ts and common.ts

```
IdlError: Type not found: {"name":"sqrtPrice","type":{"defined":"f64"}}
    at Function.fieldLayout (/home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/coder/idl.ts:89:19)
    at /home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/coder/idl.ts:117:28
    at Array.map (<anonymous>)
    at Function.typeDefLayout (/home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/coder/idl.ts:116:48)
    at /home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/coder/accounts.ts:26:39
    at Array.map (<anonymous>)
    at new AccountsCoder (/home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/coder/accounts.ts:25:49)
    at new Coder (/home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/coder/index.ts:40:21)
    at new Program (/home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/program/index.ts:264:19)
    at /home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/workspace.ts:58:34
    at Array.forEach (<anonymous>)
    at Object.get (/home/hp/Documents/cyclos/cyclos-protocol-v2/node_modules/@project-serum/anchor/src/workspace.ts:51:33)
    at Suite.<anonymous> (/home/hp/Documents/cyclos/cyclos-protocol-v2/tests/cyclos-protocol-v2.ts:11:36)
    at Object.create (/usr/local/lib/node_modules/ts-mocha/node_modules/mocha/lib/interfaces/common.js:148:19)
    at context.describe.context.context (/usr/local/lib/node_modules/ts-mocha/node_modules/mocha/lib/interfaces/bdd.js:42:27)
    at Object.<anonymous> (/home/hp/Documents/cyclos/cyclos-protocol-v2/tests/cyclos-protocol-v2.ts:6:1)
    at Module._compile (node:internal/modules/cjs/loader:1101:14)
    at Module.m._compile (/usr/local/lib/node_modules/ts-mocha/node_modules/ts-node/src/index.ts:439:23)
    at Module._extensions..js (node:internal/modules/cjs/loader:1153:10)
    at Object.require.extensions.<computed> [as .ts] (/usr/local/lib/node_modules/ts-mocha/node_modules/ts-node/src/index.ts:442:12)
    at Module.load (node:internal/modules/cjs/loader:981:32)
    at Function.Module._load (node:internal/modules/cjs/loader:822:12)
    at Module.require (node:internal/modules/cjs/loader:1005:19)
    at require (node:internal/modules/cjs/helpers:102:18)
    at Object.exports.requireOrImport (/usr/local/lib/node_modules/ts-mocha/node_modules/mocha/lib/esm-utils.js:42:12)
    at Object.exports.loadFilesAsync (/usr/local/lib/node_modules/ts-mocha/node_modules/mocha/lib/esm-utils.js:55:34)
    at singleRun (/usr/local/lib/node_modules/ts-mocha/node_modules/mocha/lib/cli/run-helpers.js:125:3)
```