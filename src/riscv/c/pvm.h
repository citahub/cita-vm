#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#ifndef _PVM_H
#define _PVM_H

static inline long
__internal_syscall(long n, long _a0, long _a1, long _a2, long _a3, long _a4, long _a5)
{
    register long a0 asm("a0") = _a0;
    register long a1 asm("a1") = _a1;
    register long a2 asm("a2") = _a2;
    register long a3 asm("a3") = _a3;
    register long a4 asm("a4") = _a4;
    register long a5 asm("a5") = _a5;
    register long syscall_id asm("a7") = n;
    asm volatile ("scall": "+r"(a0) : "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a5), "r"(syscall_id));
    return a0;
}

#define syscall(n, a, b, c, d, e, f) \
    __internal_syscall(n, (long)(a), (long)(b), (long)(c), (long)(d), (long)(e), (long)(f))


#define SYSCODE_DEBUG 2177
#define SYSCODE_RET 2180
#define SYSCODE_SAVE 2181
#define SYSCODE_LOAD 2182
#define SYSCODE_ADDRESS 2190
#define SYSCODE_BALANCE 2191
#define SYSCODE_ORIGIN 2192
#define SYSCODE_CALLER 2193
#define SYSCODE_CALLVALUE 2194
// #define CALLDATALOAD 2195
// #define CALLDATASIZE 2196
// #define CALLDATACOPY 2197
// #define CODESIZE 2198
// #define CODECOPY 2199
// #define GASPRICE 3000
// #define EXTCODESIZE 3001
// #define EXTCODECOPY 3002
// #define RETURNDATASIZE 3003
// #define RETURNDATACOPY 3004

#define SYSCODE_BLOCKHASH 3010
#define SYSCODE_COINBASE 3011
#define SYSCODE_TIMESTAMP 3012
#define SYSCODE_NUMBER 3013
#define SYSCODE_DIFFICULTY 3014
#define SYSCODE_GASLIMIT 3015


// Function pvm_debug accepts a string that contains the text to be written to stdout(It depends on the VM).
// Params:
//   format: same as the standard C function `printf()`
// Return:
//   code: 0(success)
// Example:
//   evn_debug("Hello World!");
int pvm_debug(const char* s)
{
  return syscall(SYSCODE_DEBUG, s, 0, 0, 0, 0, 0);
}

// Function ret returns any bytes to host, as the output of the current contract.
// Params:
//   data: a pointer to a buffer in VM memory space denoting where the data we are about to send.
//   size: size of the data buffer
// Return:
//   code: 0(success)
//
// Note: This syscall(s) only allowed to call once. If called it multiple times, the last call will replace the
// previous call.
int pvm_ret(uint8_t *data, size_t size)
{
    return syscall(SYSCODE_RET, data, size, 0, 0, 0, 0);
}

// Function pvm_save stores any bytes with it's keys into the global SRAM.
// Params:
//   k: a pointer to a buffer in VM memory space denoting where the key located at.
//   k_size: size of the k buffer.
//   v: a pointer to a buffer in VM memory space denoting where the value located at.
//   v_size: size of the v buffer.
// Return:
//   code: 0(success)
int pvm_save(uint8_t *k, size_t k_size, uint8_t *v, size_t v_size)
{
    return syscall(SYSCODE_SAVE, k, k_size, v, v_size, 0, 0);
}


// Function pvm_load loads bytes with given key from the global SRAM.
// Params:
//   k: a pointer to a buffer in VM memory space denoting where the key located at.
//   k_size: size of the k buffer.
//   v: a pointer to a buffer in VM memory space denoting where we would load the data.
//   v_size: size of the v buffer.
// Return:
//   code: 0(success), 1(key not found)
int pvm_load(uint8_t *k, size_t k_size, uint8_t *v, size_t v_size, size_t *r_size)
{
    return syscall(SYSCODE_LOAD, k, k_size, v, v_size, r_size, 0);
}

// Function pvm_address loads current address from context.
// Params:
//   addr: a pointer to a buffer in VM memory space denoting where the address located at.
// Return:
//   code: 0(success)
int pvm_address(uint8_t *addr)
{
    return syscall(SYSCODE_ADDRESS, addr, 0, 0, 0, 0, 0);
}

// Function pvm_balance loads balance of the specific address.
// Params:
//   addr: a pointer to a buffer in VM memory space denoting where the address located at.
//   v: a pointer to a 32 bytes buffer where the value located at.
// Return:
//   code: 0(success)
int pvm_balance(uint8_t *addr, uint8_t *v)
{
    return syscall(SYSCODE_BALANCE, addr, v, 0, 0, 0, 0);
}

// Function pvm_origin loads current origin.
// Params:
//   addr: a pointer to a buffer in VM memory space denoting where the address located at.
// Return:
//   code: 0(success)
int pvm_origin(uint8_t *addr)
{
    return syscall(SYSCODE_ORIGIN, addr, 0, 0, 0, 0, 0);
}

// Function pvm_caller loads current caller.
// Params:
//   addr: a pointer to a buffer in VM memory space denoting where the address located at.
// Return:
//   code: 0(success)
int pvm_caller(uint8_t *addr)
{
    return syscall(SYSCODE_CALLER, addr, 0, 0, 0, 0, 0);
}

// Function pvm_callvalue loads current value.
// Params:
//   v: a pointer to a 32 bytes buffer where the value located at.
// Return:
//   code: 0(success)
int pvm_callvalue(uint8_t *v)
{
    return syscall(SYSCODE_CALLVALUE, v, 0, 0, 0, 0, 0);
}

// Function pvm_blockhash loads specific block's hash.
// Params:
//   v: a pointer to a 32 bytes buffer where the hash located at.
// Return:
//   code: 0(success)
int pvm_blockhash(uint64_t h, uint8_t *hash)
{
    return syscall(SYSCODE_BLOCKHASH, h, hash, 0, 0, 0, 0);
}

// Function pvm_coinbase loads current coinbase address.
// Params:
//   addr: a pointer to a buffer in VM memory space denoting where the address located at.
// Return:
//   code: 0(success)
int pvm_coinbase(uint8_t *addr)
{
    return syscall(SYSCODE_COINBASE, addr, 0, 0, 0, 0, 0);
}

// Function pvm_timestamp loads current timestamp.
// Params:
//   time: a pointer to a uint64_t in VM memory space denoting where the timestamp located at.
// Return:
//   code: 0(success)
int pvm_timestamp(uint64_t *time)
{
    return syscall(SYSCODE_TIMESTAMP, time, 0, 0, 0, 0, 0);
}

// Function pvm_number loads current block number.
// Params:
//   v: a pointer to a 32 bytes buffer where the value located at.
// Return:
//   code: 0(success)
int pvm_number(uint8_t *number)
{
    return syscall(SYSCODE_NUMBER, number, 0, 0, 0, 0, 0);
}

// Function pvm_difficulty loads current difficulty.
// Params:
//   difficulty: a pointer to a 32 bytes buffer in VM memory space denoting where the difficulty located at.
// Return:
//   code: 0(success)
int pvm_difficulty(uint8_t *difficulty)
{
    return syscall(SYSCODE_DIFFICULTY, difficulty, 0, 0, 0, 0, 0);
}

// Function pvm_gaslimit loads current block gaslimit.
// Params:
//   gaslimit: a pointer to a uint64_t in VM memory space denoting where the gaslimit located at.
// Return:
//   code: 0(success)
int pvm_gaslimit(uint64_t *gaslimit)
{
    return syscall(SYSCODE_GASLIMIT, gaslimit, 0, 0, 0, 0, 0);
}

#endif
