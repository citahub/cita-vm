extern crate cita_evm;

use cita_evm::interpreter::stack::*;

#[test]
fn test_stack_with_capacity() {
    let mut st: Stack<u32> = Stack::with_capacity(2);
    assert_eq!(st.len(), 0);
    st.push(0x01);
    st.push(0x02);
    assert_eq!(st.len(), 2);
    st.push(0x03);
    assert_eq!(st.len(), 3);
}

#[test]
fn test_stack_new() {
    let mut st: Stack<u32> = Stack::new();
    assert_eq!(st.len(), 0);
    st.push(0x01);
    st.push(0x02);
    assert_eq!(st.len(), 2);
    st.push(0x03);
    assert_eq!(st.len(), 3);
}

#[test]
fn test_stack_back() {
    let mut st: Stack<u32> = Stack::new();
    st.push(0x01);
    st.push(0x02);
    assert_eq!(st.back(0), 0x02);
    assert_eq!(st.back(1), 0x01);
}

#[test]
#[should_panic]
fn test_stack_back_neg() {
    let mut st: Stack<u32> = Stack::new();
    st.push(0x01);
    st.push(0x02);
    st.back(2);
}

#[test]
fn test_stack_push_n() {
    let mut st: Stack<u32> = Stack::new();
    let ls = vec![0x01, 0x02];
    st.push_n(&ls);
    assert_eq!(st.back(0), 0x02);
    assert_eq!(st.back(1), 0x01);
}

#[test]
fn test_stack_pop() {
    let mut st: Stack<u32> = Stack::new();
    let ls = vec![0x01, 0x02];
    st.push_n(&ls);
    assert_eq!(st.pop(), 0x02);
    assert_eq!(st.pop(), 0x01);
}


#[test]
#[should_panic]
fn test_stack_pop_neg() {
    let mut st: Stack<u32> = Stack::new();
    let ls = vec![0x01, 0x02];
    st.push_n(&ls);
    for i in 0..3 {
        println!("{}", i);
        st.pop();
    }
}

#[test]
fn test_stack_swap() {
    let mut st: Stack<u32> = Stack::new();
    let ls = vec![0x01, 0x02, 0x03, 0x04];
    st.push_n(&ls);
    st.swap(2);
    assert_eq!(st.back(0), 0x02);
    assert_eq!(st.back(2), 0x04);
}

#[test]
fn test_stack_dup() {
    let mut st: Stack<u32> = Stack::new();
    let ls = vec![0x01, 0x02];
    st.push_n(&ls);
    st.dup(0);
    assert_eq!(st.back(0), 0x02);
    st.dup(2);
    assert_eq!(st.back(0), 0x01);
}
