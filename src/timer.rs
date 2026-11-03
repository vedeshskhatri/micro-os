#[derive(Debug)]
pub struct Timer {
    pub ticks: usize,
    pub quantum: usize,
    elapsed_in_quantum: usize,
}

impl Timer {
    pub fn new(quantum: usize) -> Self {
        assert!(quantum > 0, "timer quantum must be positive");
        Self {
            ticks: 0,
            quantum,
            elapsed_in_quantum: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.ticks += 1;
        self.elapsed_in_quantum += 1;
        if self.elapsed_in_quantum == self.quantum {
            self.elapsed_in_quantum = 0;
            true
        } else {
            false
        }
    }
}
