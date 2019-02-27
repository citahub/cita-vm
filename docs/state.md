# State 文档

重要结构体介绍:

- Account: 基本结构体, 包含 `balance`, `nonce`, `storage_root` 和 `code_hash`, 用于序列化与反序列化
- StateObject: 对 Account 的封装, 表示该 account 正在被 EVM 使用. 包含诸如 get/set_balance, get/set_storage 等方法
- StateObjectEntry: 对 StateObject 的封装, 包含一个字段表面 StateObject 的内容是否发生过修改. (dirty or clean).
- State: 管理 StateObjectEntries, 外界应该只关心 state 对象和它提供的方法.

下面我们将直接从 State 对象入手介绍 state 模块.

# 底层数据

数据库存储三类信息:

- account: rlp(nonce, balance, storage_root, code_hash)
- account.storage_root: storage_trie
- account.code_hash: code

注意的是 world trie root 并没有存储在该数据库中, 因此使用者需要自己想办法存储 world trie root(一个好的想法是存在 DB for blocks 中).

# 缓存设计

State 层存在一层账号级别的缓存, 数据库层存在一层 KV 级别的缓存. 由于数据库并未在该模块实现, 因此使用者在使用 state 的时候, 需要自己实现 KV 层缓存. 先介绍 State 层缓存: State 层缓存作用是缓存复数个账号实例.

```rs
pub cache: RefCell<HashMap<Address, StateObjectEntry>>,
```

- 首先所有 Dirty 的操作都是先写入缓存, 只有通过 `commit()` 函数才会写入真实数据库
- 使用 `clear()` 清空缓存. Note: 为什么没有使用 LRU? 因为缓存中同时保存了状态修改, 使用 LRU 会发生不可控的缓存刷新导致丢失状态修改, 因此 state 的使用者必须自己决定什么时候调用 `clear()`.

# Dirty / Clean ?

判断一个 StateObject 是否是 Dirty 的, 只需关心它从数据库读取到 State 对象后, 它所包含的数据内容是否发生了变化. 因此, 以下功能将使 StateObject 的状态从 Clean 修改为 Dirty:

- kill_contract
- set_storage
- add_balance
- sub_balance
- incr_nonce
- create_new_contract
- set_code

# Checkpoint

Checkpoint 是 State 提供的最重要的功能. checkpoint 功能是创建 State 缓存的(多个)快照, 并在需要的时候回滚缓存到指定时间点.

先看下 checkpoints 的定义:

```rs
pub checkpoints: RefCell<Vec<HashMap<Address, Option<StateObjectEntry>>>>,
```

首先它定义为一个数组, 表示可以同时存在多个 checkpoint, 数组的每一项 `HashMap<Address, Option<StateObjectEntry>` 表示一个 checkpoint 可以缓存多个账号与数据信息.

## 创建 checkpoint

Checkpoint 的创建是 Lazy 的. 它只会往 checkpoints 中添加一个空的 HashMap.

```rs
/// Create a recoverable checkpoint of this state. Return the checkpoint index.
pub fn checkpoint(&mut self) -> usize {
    let mut checkpoints = self.checkpoints.borrow_mut();
    let index = checkpoints.len();
    checkpoints.push(HashMap::new());
    index
}
```

**在创建 checkpoint 后对某个账号进行的第一次 Dirty 操作, 将把操作前的 StateObjectEntry 写入 checkpoints 列表的最后一项**

假设初始账号 A 拥有一个 (Key: Value0), 创建 checkpoint 后先后 set(Key, Value1) 和 set(Key, Value2), 则在最近的 checkpoint 中存储的值是 Key: Value0

## 合并 checkpoint

合并最后一个 checkpoint 到之前的 checkpoint, 最关键的部分是 **checkpoint 总保留最老的数据**.

```rs
/// Merge last checkpoint with previous.
pub fn discard_checkpoint(&mut self) {
    let last = self.checkpoints.borrow_mut().pop();
    if let Some(mut checkpoint) = last {
        if let Some(prev) = self.checkpoints.borrow_mut().last_mut() {
            if prev.is_empty() {
                *prev = checkpoint;
            } else {
                for (k, v) in checkpoint.drain() {
                    prev.entry(k).or_insert(v); // checkpoint 总保留最老的数据
                }
            }
        }
    }
}
```

## checkpoint 回滚

回滚 checkpoint 的关键是将 checkpoint 内的数据刷回缓存(因为在 commit 之前, 所有 Dirty 的修改都保存在缓存中)

```rs
/// Revert to the last checkpoint and discard it.
pub fn revert_checkpoint(&mut self) {
    if let Some(mut last) = self.checkpoints.borrow_mut().pop() {
        for (k, v) in last.drain() {
            match v {
                Some(v) => match self.cache.get_mut().entry(k) {
                    Entry::Occupied(mut e) => {
                        // Merge checkpointed changes back into the main account
                        // storage preserving the cache.
                        e.get_mut().merge(v);
                    }
                    Entry::Vacant(e) => {
                        e.insert(v);
                    }
                },
                None => {
                    if let Entry::Occupied(e) = self.cache.get_mut().entry(k) {
                        if e.get().is_dirty() {
                            e.remove();
                        }
                    }
                }
            }
        }
    }
}
```
