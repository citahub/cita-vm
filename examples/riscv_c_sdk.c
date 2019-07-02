#include <string.h>

#include "pvm.h"

int main(int argc, char* argv[]) {
  pvm_debug("Testing: debug");
  pvm_debug("Test[v]: debug");

  pvm_debug("Testing: ret");
  uint8_t *buffer_ret = (uint8_t *)"Test: ret";
  pvm_ret(&buffer_ret[0], strlen(buffer_ret));
  pvm_debug("Test[v]: ret");

  pvm_debug("Testing: save");
  uint8_t *buffer_save_k = (uint8_t *)"Test: save_k";
  uint8_t *buffer_save_v = (uint8_t *)"Test: save_v";
  pvm_save(&buffer_save_k[0], strlen(buffer_save_k), &buffer_save_v[0], strlen(buffer_save_v));
  pvm_debug("Test[v]: save");

  pvm_debug("Testing: load");
  uint8_t buffer_load_v[20];
  size_t sz;
  pvm_load(&buffer_save_k[0], strlen(buffer_save_k), &buffer_load_v[0], 20, &sz);
  const char* s = buffer_load_v;
  if ((strcmp("Test: save_v", s) == 0) && (sz == 12)) {
    pvm_debug("Test[v]: load");
  } else {
    pvm_debug("Test[x]: load");
  }

  pvm_debug("Testing: address");
  uint8_t addr[20];
  pvm_address(&addr[0]);
  if (addr[19] == 0x01) {
    pvm_debug("Test[v]: address");
  } else {
    pvm_debug("Test[x]: address");
  }

  pvm_debug("Testing: balance");
  uint8_t account1[20] = {
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
  };
  uint8_t v[32];
  pvm_balance(&account1[0], &v[0]);
  if (v[31] == 10) {
    pvm_debug("Test[v]: balance");
  } else {
    pvm_debug("Test[x]: balance");
  }

  pvm_debug("Testing: origin");
  uint8_t origin[20];
  pvm_origin(&origin[0]);
  if (origin[19] == 0x02) {
    pvm_debug("Test[v]: origin");
  } else {
    pvm_debug("Test[x]: origin");
  }

  pvm_debug("Testing: caller");
  uint8_t caller[20];
  pvm_caller(&caller[0]);
  if (caller[19] == 0x03) {
    pvm_debug("Test[v]: caller");
  } else {
    pvm_debug("Test[x]: caller");
  }

  pvm_debug("Testing: callvalue");
  uint8_t callvalue[32];
  pvm_callvalue(&callvalue[0]);
  if (callvalue[31] == 5) {
    pvm_debug("Test[v]: callvalue");
  } else {
    pvm_debug("Test[x]: callvalue");
  }

  pvm_debug("Testing: block hash");
  uint8_t block_hash[32];
  pvm_blockhash(7, &block_hash[0]);
  if (block_hash[31] == 7) {
    pvm_debug("Test[v]: block hash");
  } else {
    pvm_debug("Test[x]: block hash");
  }

  pvm_debug("Testing: coinbase");
  uint8_t coinbase[20];
  pvm_coinbase(&coinbase[0]);
  if (coinbase[19] == 0x08) {
    pvm_debug("Test[v]: coinbase");
  } else {
    pvm_debug("Test[x]: coinbase");
  }

  pvm_debug("Testing: timestamp");
  uint64_t timestamp;
  pvm_timestamp(&timestamp);
  if (timestamp == 0x09) {
    pvm_debug("Test[v]: timestamp");
  } else {
    pvm_debug("Test[x]: timestamp");
  }

  pvm_debug("Testing: number");
  uint8_t number[32];
  pvm_number(&number[0]);
  if (number[31] == 0x06) {
    pvm_debug("Test[v]: number");
  } else {
    pvm_debug("Test[x]: number");
  }

  pvm_debug("Testing: difficulty");
  uint8_t difficulty[32];
  pvm_difficulty(&difficulty[0]);
  if (difficulty[31] == 0x0a) {
    pvm_debug("Test[v]: difficulty");
  } else {
    pvm_debug("Test[x]: difficulty");
  }

  return 0;
}
