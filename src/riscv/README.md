# RISCV Executor

Similar to EVM, but using the RISCV instruction set.

```
+-----------------+      +------+
| RISCV Interpter | <--> | EVM  |
+-----------------+      +------+
     |
+------------------------+
| Account model Storage  |
+------------------------+
     |
+----------+
|   MPT    |
+----------+
```

# RISCV Contract Model: C-Language

C-Language contract is very simaliar with native binary program. The minimal contract is like this:

```c
int main(int argc, char* argv[]) {
    return 0;
}
```

If you want to interact with data on the chain, The only way you should do is that include our SDK. The following code implements a simple data storage. The core functions are: `pvm_save` and `pvm_load`, It allows your play with the **World State**.

```c
#include <string.h>

#include "pvm.h"
#include "pvm_extend.h"

int main(int argc, char* argv[]) {
  if (argc == 1) {
    return 0;
  }

  if (strcmp(argv[1], "get") == 0) {
    if (argc != 3) {
      return 1;
    }
    uint8_t *k = (uint8_t *)argv[2];
    uint8_t v[8];
    pvm_load(&k[0], strlen(k), &v[0], 8, NULL);
    uint64_t r = pvm_decode_u64(&v[0]);
    pvm_ret_u64(r);
    return 0;
  }

  if (strcmp(argv[1], "set") == 0) {
    if (argc != 4) {
      return 1;
    }
    uint8_t *k = (uint8_t *)argv[2];
    uint8_t v[8];
    pvm_encode_u64(&v[0], atoi(argv[3]));
    pvm_save(&k[0], strlen(k), &v[0], 8);
    return 0;
  }
  return 0;
}
```

# Return data from the call

Contract allows return a byte array by function `pvm_ret`. You can stuff any data in it, as long as you are happy. But still recommend that you use some standards like use 8 bytes describe a uint64 number.

```c
int pvm_ret(uint8_t *data, size_t size);
```

# Error handling

Like with C, returns a number that not equals with zero means something went wrong.

```c
#define ERROR_HAPPENED 1

int main(int argc, char* argv[]) {
    if error_happened {
        return ERROR_HAPPENED
    }
}
```

You can add a extra message by `pvm_ret`.

# RISCV Contract Model: JS-Language

**Duktape** is used for interpret JS-Language contract. The minimal contract is like

```js
function main(argc, argv) {
}
```

Similar to C, isn't it?

There are a global object named `pvm` that allows you play with the World State:

```js
// Another data storage contract write by JS
function main(argc, argv) {
    if (argv[1] == 'set') {
        var k = argv[2]
        var v = new Buffer(argv[3])
        pvm.save(k, v)
    }
    if (argv[1] == 'get') {
        var k = argv[2]
        var v = pvm.load(k)
        pvm.ret(v)
    }
}
```

# Functions in SDK

```c
int pvm_debug(const char* s)
int pvm_ret(uint8_t *data, size_t size)
int pvm_save(uint8_t *k, size_t k_size, uint8_t *v, size_t v_size)
int pvm_load(uint8_t *k, size_t k_size, uint8_t *v, size_t v_size, size_t *r_size)
int pvm_address(uint8_t *addr)
int pvm_balance(uint8_t *addr, uint8_t *v)
int pvm_origin(uint8_t *addr)
int pvm_caller(uint8_t *addr)
int pvm_callvalue(uint8_t *v)
int pvm_blockhash(uint64_t h, uint8_t *hash)
int pvm_coinbase(uint8_t *addr)
int pvm_timestamp(uint64_t *time)
int pvm_number(uint8_t *number)
int pvm_difficulty(uint8_t *difficulty)
int pvm_gaslimit(uint64_t *gaslimit)
```

or in JS's `pvm` object, just use `pvm.xxxxx` instead of `pvm_xxxxx`.

# Data structure

I am thinking about whether to add built-in types, such as **Address**, **Balance**, e.g. But for now, I use

- Address: `[20]uint8_t`
- Balance: `[32]uint8_t`, No build-in U256!

The more advanced types maybe provided in the **develop toolchain**, but should not be in the vm layer.

# Runtime Cost

The gas cost table is copied from CKB.

C-Language contract not much different from the EVM. If we don't consider about the gas cost for `set_storage` and `get_storage`, only `1708` and `1665` used in riscv vm(simplestorage contract's `set` and `get` function).

But when calling JS contract, the most simple data storage contract cost **~7000000** gas(It's sure we can do a lot of optimization here, but still much bigger than C or EVM).

In addition, I recommend using C-Language to write the contract code.
