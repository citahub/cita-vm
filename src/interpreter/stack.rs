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
        self.data.swap(len - n - 1, len - 1)
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
