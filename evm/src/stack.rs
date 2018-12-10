pub struct Stack<T> {
    data: Vec<T>,
}

impl<T: Clone + Copy> Stack<T> {
    pub fn new() -> Stack<T> {
        Stack { data: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Stack {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn push(&mut self, d: T) {
        self.data.push(d)
    }

    pub fn push_n(&mut self, ds: &[T]) {
        self.data.extend_from_slice(ds)
    }

    pub fn pop(&mut self) -> T {
        self.data.pop().unwrap()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn swap(&mut self, n: usize) {
        let len = self.data.len();
        self.data.swap(len - 1 - n, len - 1)
    }

    pub fn dup(&mut self, n: usize) {
        let d = self.back(n);
        self.data.push(d)
    }

    pub fn back(&self, n: usize) -> T {
        self.data[self.data.len() - n - 1].clone()
    }

    pub fn peek(&self) -> T {
        self.back(0)
    }

    pub fn require(&self, n: usize) -> bool {
        self.data.len() >= n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let ls = vec![0x01, 0x02, 0x03, 0x04];
        st.push_n(&ls);
        st.dup(1);
        assert_eq!(st.back(0), 0x03);
        st.dup(3);
        assert_eq!(st.back(0), 0x02);
    }
}
