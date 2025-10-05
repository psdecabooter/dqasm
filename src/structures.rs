use std::collections::HashSet;

pub struct Header {
    pub magic: [u8; 4],
    pub version: u32,
    pub num_qubits: u32,
    pub num_gates: u64,
}

/// Gate types:
///
/// 0: t
///
/// 1: tdg
///
/// 2: cx
pub struct Gate {
    pub gate_type: u8,
    pub qubit1: u32,
    pub qubit2: u32,
}
impl Gate {
    fn new(gate_type: u8, qubit1: u32, qubit2: u32) -> Self {
        Gate {
            gate_type,
            qubit1,
            qubit2,
        }
    }
    pub fn t(q: u32) -> Self {
        Gate::new(0, q, 0)
    }

    pub fn tdg(q: u32) -> Self {
        Gate::new(1, q, 0)
    }

    pub fn cx(q1: u32, q2: u32) -> Self {
        Gate::new(2, q1, q2)
    }

    pub fn is_double_qubit(&self) -> bool {
        matches!(self.gate_type, 2)
    }

    pub fn get_qubits(&self) -> (u32, Option<u32>) {
        match self.is_double_qubit() {
            true => (self.qubit1, None),
            false => (self.qubit1, Some(self.qubit2)),
        }
    }
}

pub struct Circuit {
    pub gates: Vec<Gate>,
    pub qubits: HashSet<u32>,
}
impl Circuit {
    pub fn new() -> Self {
        Circuit {
            gates: Vec::new(),
            qubits: HashSet::new(),
        }
    }
    pub fn add_gate(&mut self, gate: Gate) {
        let (q0, maybe_q1) = gate.get_qubits();
        if !self.qubits.contains(&q0) {
            self.qubits.insert(q0);
        }
        if let Some(q1) = maybe_q1 {
            if !self.qubits.contains(&q1) {
                self.qubits.insert(q1);
            }
        }
        self.gates.push(gate);
    }
}
