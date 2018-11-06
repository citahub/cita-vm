pub struct Stack<T> {
    data: Vec<T>,
}

impl<T: Clone + Copy> Stack<T> {
    pub fn new(capacity: usize) -> Self {
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
        match self.data.pop() {
            Some(v) => v,
            None => panic!("Tried to pop from empty stack."),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn swap(&mut self, n: usize) {
        let len = self.data.len();
        self.data.swap(len - n - 1, len - 1)
    }

    pub fn dup(&mut self, n: usize) {
        let d = self.data[n];
        self.data.push(d)
    }

    pub fn peek(&self) -> &T {
        &self.data[self.data.len() - 1]
    }

    pub fn back(&self, n: usize) -> T {
        match self.data.get(self.data.len() - n - 1) {
            Some(v) => v.clone(),
            None => panic!("Tried to back from empty stack."),
        }
    }

    pub fn require(&self, n: usize) -> bool {
        self.data.len() > n
    }
}
