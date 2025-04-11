#[derive(Debug, Clone)]
pub struct CycleCount {
    cycles: Vec<usize>,
    lookup: String,
    reg_count: usize,
}

impl CycleCount {
    pub fn new(cycles: Vec<usize>, lookup: String, reg_count: usize) -> Self {
        CycleCount {
            cycles,
            lookup,
            reg_count,
        }
    }

    pub fn base(&self) -> usize {
        self.cycles.first().cloned().unwrap_or(0)
    }

    pub fn cycles_per_reg(&self) -> usize {
        self.cycles.get(1).cloned().unwrap_or(0)
    }

    pub fn get_reg_count(&self) -> usize {
        self.reg_count
    }

    pub fn get_lookup(&self) -> String {
        self.lookup.clone()
    }

    pub fn get_cycles(&self) -> Vec<usize> {
        self.cycles.clone()
    }

    pub fn extra_if_taken(&self) -> usize {
        self.cycles.get(1).cloned().unwrap_or(0)
    }

    pub fn _total_taken(&self, iterations: usize) -> usize {
        if iterations <= 1 {
            self.base()
        } else {
            (iterations - 1) * self.extra_if_taken() + self.base()
        }
    }
}
